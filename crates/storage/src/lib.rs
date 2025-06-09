use std::collections::VecDeque;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ffmpeg_next::ffi::AVCodecParameters;
use ffmpeg_next::{codec, sys};

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
    max_size_bytes: usize,
    current_size: Arc<Mutex<usize>>,
}

impl ReplayBuffer {
    pub fn new(buffer_duration_secs: u32, max_size_mb: usize) -> Self {
        Self {
            packets: Arc::new(Mutex::new(VecDeque::new())),
            max_duration: Duration::from_secs(buffer_duration_secs as u64),
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size: Arc::new(Mutex::new(0)),
        }
    }

    pub fn add_packet(&self, data: Vec<u8>, is_keyframe: bool) {
        let packet = TimestampedPacket {
            data,
            timestamp: Instant::now(),
            is_keyframe,
        };

        let mut packets = self.packets.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();

        *current_size += packet.data.len();
        packets.push_back(packet);

        let cutoff_time = Instant::now() - self.max_duration;
        while let Some(front) = packets.front() {
            if front.timestamp < cutoff_time || *current_size > self.max_size_bytes {
                let removed = packets.pop_front().unwrap();
                *current_size -= removed.data.len();
            } else {
                break;
            }
        }
    }

    pub fn save_to_file(
        &self,
        output_path: &str,
        codecpar: *mut AVCodecParameters,
    ) -> Result<(), String> {
        let packets = self.packets.lock().unwrap();
        if packets.is_empty() {
            return Err("Replay buffer is empty".to_string());
        }

        // Find the first keyframe
        let first_keyframe = packets.iter().position(|p| p.is_keyframe);
        if first_keyframe.is_none() {
            return Err("No keyframe found in replay buffer".to_string());
        }

        unsafe {
            use std::ffi::CString;
            let c_output_path =
                CString::new(output_path).map_err(|_| "Invalid output path".to_string())?;
            let mut format_ctx: *mut sys::AVFormatContext = ptr::null_mut();
            let ret = sys::avformat_alloc_output_context2(
                &mut format_ctx,
                ptr::null_mut(),
                ptr::null(),
                c_output_path.as_ptr(),
            );
            if ret < 0 || format_ctx.is_null() {
                return Err(format!("Failed to allocate output context: {}", ret));
            }

            let stream = sys::avformat_new_stream(format_ctx, ptr::null_mut());
            if stream.is_null() {
                sys::avformat_free_context(format_ctx);
                return Err("Failed to create new stream".to_string());
            }

            // Copy codec parameters from input
            if sys::avcodec_parameters_copy((*stream).codecpar, codecpar) < 0 {
                sys::avformat_free_context(format_ctx);
                return Err("Failed to copy codec parameters".to_string());
            }

            // Set a default time_base (e.g., 1/30 for 30 fps)
            (*stream).time_base = sys::AVRational { num: 1, den: 30 };

            // Open output file if needed
            if (*(*format_ctx).oformat).flags & sys::AVFMT_NOFILE == 0 {
                let mut pb: *mut sys::AVIOContext = ptr::null_mut();
                if sys::avio_open(&mut pb, c_output_path.as_ptr(), sys::AVIO_FLAG_WRITE) < 0 {
                    sys::avformat_free_context(format_ctx);
                    return Err("Failed to open output file".to_string());
                }
                (*format_ctx).pb = pb;
            }

            if sys::avformat_write_header(format_ctx, ptr::null_mut()) < 0 {
                if (*(*format_ctx).oformat).flags & sys::AVFMT_NOFILE == 0 {
                    sys::avio_closep(&mut (*format_ctx).pb);
                }
                sys::avformat_free_context(format_ctx);
                return Err("Failed to write header".to_string());
            }

            // Write packets, set pts/dts incrementally
            let mut pts = 0;
            let mut dts = 0;
            let duration = 1; // You may want to set this based on fps
            for packet in packets.iter().skip(first_keyframe.unwrap()) {
                let mut av_packet = sys::AVPacket {
                    data: packet.data.as_ptr() as *mut u8,
                    size: packet.data.len() as i32,
                    pts,
                    dts,
                    duration,
                    stream_index: 0,
                    flags: if packet.is_keyframe {
                        sys::AV_PKT_FLAG_KEY
                    } else {
                        0
                    },
                    ..std::mem::zeroed()
                };
                pts += 1;
                dts += 1;

                if sys::av_interleaved_write_frame(format_ctx, &mut av_packet) < 0 {
                    if (*(*format_ctx).oformat).flags & sys::AVFMT_NOFILE == 0 {
                        sys::avio_closep(&mut (*format_ctx).pb);
                    }
                    sys::avformat_free_context(format_ctx);
                    return Err("Failed to write packet".to_string());
                }
            }

            sys::av_write_trailer(format_ctx);
            if (*(*format_ctx).oformat).flags & sys::AVFMT_NOFILE == 0 {
                sys::avio_closep(&mut (*format_ctx).pb);
            }
            sys::avformat_free_context(format_ctx);
        }
        Ok(())
    }
}
