use log::{info, warn}; // Make sure info and warn are imported
use std::collections::VecDeque;
use std::ffi::CString;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ffmpeg_next::sys;

#[derive(Clone)]
#[repr(C)]
pub struct TimestampedPacket {
    pub data: Vec<u8>,
    pub timestamp: Instant,
    pub is_keyframe: bool,
}

pub struct ReplayBuffer {
    packets: Arc<Mutex<VecDeque<TimestampedPacket>>>,
    max_duration: Duration,
}

impl ReplayBuffer {
    pub fn new(buffer_duration_secs: u32, _estimated_packets: usize) -> Self {
        let buffer = Self {
            packets: Arc::new(Mutex::new(VecDeque::new())),
            max_duration: Duration::from_secs(buffer_duration_secs as u64),
        };
        info!("[storage] ReplayBuffer created with max duration: {:?}", buffer.max_duration);
        buffer
    }

    pub fn add_packet(&self, data: Vec<u8>, is_keyframe: bool) {
        let packet = TimestampedPacket {
            data,
            timestamp: Instant::now(),
            is_keyframe,
        };

        let mut packets = self.packets.lock().unwrap();
        let before_len = packets.len();
        packets.push_back(packet);

        let cutoff_time = match Instant::now().checked_sub(self.max_duration) {
            Some(time) => time,
            None => return,
        };

        // --- Start of Pruning Logic ---
        // Find the index of the first packet that is NEWER than our cutoff time.
        let first_valid_index = packets.iter().position(|p| p.timestamp >= cutoff_time);

        if let Some(start_idx) = first_valid_index {
            // We found the start of the "good" data. Now find the last keyframe before it.
            let last_keyframe_before_valid = packets.iter()
                .take(start_idx + 1)
                .rposition(|p| p.is_keyframe);

            if let Some(prune_until_idx) = last_keyframe_before_valid {
                if prune_until_idx > 0 {
                    packets.drain(0..prune_until_idx);
                    info!(
                        "[storage] Pruning: size before: {}, size after: {}. Drained {} packets.",
                        before_len + 1,
                        packets.len(),
                        prune_until_idx
                    );
                }
            }
        } else {
             // This case means ALL packets are older than the cutoff time.
             // We should keep only the very last GOP.
            if let Some(last_key_idx) = packets.iter().rposition(|p| p.is_keyframe) {
                if last_key_idx > 0 {
                    packets.drain(0..last_key_idx);
                     info!(
                        "[storage] Pruning ALL old: size before: {}, size after: {}. Drained {} packets.",
                        before_len + 1,
                        packets.len(),
                        last_key_idx
                    );
                }
            }
        }
    }


// This is the final version that implements the "save last X seconds" feature.
pub fn save_to_file(
    &self,
    output_path: &str,
    codecpar: *mut sys::AVCodecParameters,
    fps: u32,
) -> Result<(), String> {
    const REPLAY_DURATION_SECS: u64 = 15; // Save the last 15 seconds

    let packets_guard = self.packets.lock().unwrap();
    if packets_guard.is_empty() {
        return Err("Replay buffer is empty".to_string());
    }

    let replay_duration = Duration::from_secs(REPLAY_DURATION_SECS);
    let cutoff_time = match Instant::now().checked_sub(replay_duration) {
        Some(time) => time,
        None => packets_guard.front().unwrap().timestamp,
    };

    let window_start_index = packets_guard
        .iter()
        .position(|p| p.timestamp >= cutoff_time)
        .unwrap_or(0);

    let first_keyframe_in_window = packets_guard
        .iter()
        .skip(window_start_index)
        .position(|p| p.is_keyframe);
    
    let final_slice_start = match first_keyframe_in_window {
        Some(relative_idx) => window_start_index + relative_idx,
        None => packets_guard.iter().rposition(|p| p.is_keyframe).unwrap_or(0),
    };

    let packets_to_save: Vec<TimestampedPacket> =
        packets_guard.iter().skip(final_slice_start).cloned().collect();
    drop(packets_guard);

    unsafe {
        let c_output_path = CString::new(output_path).map_err(|_| "Invalid output path".to_string())?;
        let mut format_ctx: *mut sys::AVFormatContext = ptr::null_mut();

        sys::avformat_alloc_output_context2(
            &mut format_ctx,
            ptr::null_mut(),
            ptr::null(),
            c_output_path.as_ptr(),
        );
        if format_ctx.is_null() {
            return Err("Failed to allocate output context".to_string());
        }

        let stream = sys::avformat_new_stream(format_ctx, ptr::null_mut());
        if stream.is_null() {
            sys::avformat_free_context(format_ctx);
            return Err("Failed to create new stream".to_string());
        }

        if sys::avcodec_parameters_copy((*stream).codecpar, codecpar) < 0 {
            sys::avformat_free_context(format_ctx);
            return Err("Failed to copy codec parameters".to_string());
        }
        
        // --- KEY CHANGE: DO NOT SET THE STREAM TIME_BASE MANUALLY ---
        // We will let the muxer use its own preferred time_base (e.g. 1/15360 or 1/90000)

        if (*(*format_ctx).oformat).flags & sys::AVFMT_NOFILE == 0 {
            if sys::avio_open(&mut (*format_ctx).pb, c_output_path.as_ptr(), sys::AVIO_FLAG_WRITE) < 0 {
                sys::avformat_free_context(format_ctx);
                return Err("Failed to open output file".to_string());
            }
        }

        let mut opts: *mut sys::AVDictionary = ptr::null_mut();
        sys::av_dict_set(&mut opts, CString::new("movflags").unwrap().as_ptr(), CString::new("faststart").unwrap().as_ptr(), 0);

        if sys::avformat_write_header(format_ctx, &mut opts) < 0 {
            sys::avformat_free_context(format_ctx);
            return Err("Failed to write header".to_string());
        }
        
        // --- KEY CHANGE: DEFINE OUR SOURCE TIME_BASE AND RESCALE ---
        // 1. Define the time_base of our simple frame counter (pts_count)
        let source_time_base = sys::AVRational { num: 1, den: fps as i32 };
        let mut pts_count: i64 = 0;

        for packet_to_save in &packets_to_save {
            let mut av_packet = sys::av_packet_alloc();
            if av_packet.is_null() { continue; }

            sys::av_new_packet(av_packet, packet_to_save.data.len() as i32);
            ptr::copy_nonoverlapping(
                packet_to_save.data.as_ptr(),
                (*av_packet).data,
                packet_to_save.data.len(),
            );

            (*av_packet).flags = if packet_to_save.is_keyframe { sys::AV_PKT_FLAG_KEY as i32 } else { 0 };
            (*av_packet).stream_index = (*stream).index;
            
            // 2. Set the packet's timestamps based on the simple frame count
            (*av_packet).pts = pts_count;
            (*av_packet).dts = pts_count;
            (*av_packet).duration = 1; // Duration is 1 tick in the SOURCE time_base
            
            pts_count += 1;

            // 3. Rescale the timestamps from our source (1/fps) to the stream's destination time_base
            sys::av_packet_rescale_ts(
                av_packet,
                source_time_base,      // From
                (*stream).time_base,   // To
            );
            
            if sys::av_interleaved_write_frame(format_ctx, av_packet) < 0 {
                warn!("[storage] Failed to write a packet during save.");
            }

            sys::av_packet_free(&mut av_packet);
        }

        sys::av_write_trailer(format_ctx);
        if (*(*format_ctx).oformat).flags & sys::AVFMT_NOFILE == 0 {
            sys::avio_closep(&mut (*format_ctx).pb);
        }
        sys::avformat_free_context(format_ctx);
    }

    info!("[storage] Successfully saved replay to {}", output_path);
    Ok(())
}
}