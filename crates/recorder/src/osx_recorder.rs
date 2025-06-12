use std::ptr;
use std::sync::Arc;

use common::async_trait::async_trait;
use common::log::{error, info};
use common::tokio::sync::Mutex;

use super::recorder::Recorder;
use crate::codecpar::CodecParPtr;
use common::avdict::AVDict;
use common::cstring;
use common::sys;
use storage::ReplayBuffer;

type ArcM<T> = Arc<Mutex<T>>;

pub struct OsxRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    stop_signal: ArcM<bool>,
    replay_buffer: Arc<ReplayBuffer>,
    codecpar: std::sync::Arc<std::sync::Mutex<Option<CodecParPtr>>>,
}

// SAFETY: We ensure the pointer is only used while valid.
unsafe impl Send for OsxRecorder {}
unsafe impl Sync for OsxRecorder {}

#[async_trait]
impl Recorder for OsxRecorder {
    fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, output: String) -> Self {
        // Initialize FFmpeg
        unsafe { sys::avdevice_register_all() };

        let estimated_packets = (fps as usize) * (buffer_secs as usize) * 2;
        let replay_buffer = Arc::new(ReplayBuffer::new(buffer_secs, estimated_packets));

        Self {
            width,
            height,
            fps,
            buffer_secs,
            output,
            stop_signal: Arc::new(Mutex::new(false)),
            replay_buffer,
            codecpar: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    async fn start(&mut self) {
        *self.stop_signal.lock().await = false;

        let stop = self.stop_signal.clone();
        let buf = self.replay_buffer.clone();
        let width = self.width;
        let height = self.height;
        let fps = self.fps;
        let secs = self.buffer_secs;
        let codecpar_arc = self.codecpar.clone();

        common::tokio::task::spawn_blocking(move || {
            capture_encode_loop_sys(width, height, fps, secs, buf, stop, codecpar_arc);
        });

        info!("[recorder] macOS capture thread started");
    }

    async fn stop(&mut self) {
        *self.stop_signal.lock().await = true;
    }

    fn save(&self, final_output_path: &str) -> Result<(), String> {
        let codecpar = self.codecpar.lock().unwrap();
        let codecpar = codecpar.ok_or("Codec parameters not set")?.0;
        info!("[recorder] Saving replay buffer to {}", final_output_path);
        self.replay_buffer
            .save_to_file(final_output_path, codecpar, self.fps)
    }

    fn get_output_path(&self) -> &str {
        &self.output
    }
}

#[allow(unused_assignments)]
fn capture_encode_loop_sys(
    width: u32,
    height: u32,
    fps: u32,
    _buffer_secs: u32,
    replay_buffer: Arc<ReplayBuffer>,
    stop_signal: ArcM<bool>,
    codecpar_arc: std::sync::Arc<std::sync::Mutex<Option<CodecParPtr>>>,
) {
    unsafe {
        let mut fmt_ctx: *mut sys::AVFormatContext = ptr::null_mut();
        let mut dec_ctx: *mut sys::AVCodecContext = ptr::null_mut();
        let mut enc_ctx: *mut sys::AVCodecContext = ptr::null_mut();
        let mut scaler_ctx: *mut sys::SwsContext = ptr::null_mut();
        let mut packet: *mut sys::AVPacket = ptr::null_mut();
        let mut decoded_frame: *mut sys::AVFrame = ptr::null_mut();
        let mut scaled_frame: *mut sys::AVFrame = ptr::null_mut();

        // Use AVFoundation input format for macOS
        let input_format = sys::av_find_input_format(cstring!("avfoundation").as_ptr());
        if input_format.is_null() {
            error!("[recorder] Failed to find avfoundation input format");
            return;
        }

        let mut dict = AVDict::new();
        dict.set("framerate", &fps.to_string());
        dict.set("video_size", &format!("{}x{}", width, height));
        dict.set("pixel_format", "uyvy422"); // Common format for macOS screen capture
        dict.set("capture_cursor", "1"); // Include cursor in capture
        dict.set("capture_mouse_clicks", "1"); // Capture mouse clicks

        // For macOS, we capture the main display using AVFoundation
        // IMPORTANT: Device indices can vary between systems. Common patterns:
        // - "0:" = First camera (FaceTime HD Camera)
        // - "1:" = Screen capture (Capture screen 0)
        // - "2:" = Second screen if available
        //
        // To find correct device indices on your system, run:
        // ffmpeg -f avfoundation -list_devices true -i ""
        //
        // If screen recording fails, check:
        // 1. System Preferences > Security & Privacy > Privacy > Screen Recording
        // 2. Grant permission to your terminal/application
        // 3. Restart terminal after granting permission
        let url = cstring!("1:");
        info!(
            "[recorder] Attempting to capture screen (device 1) at {}x{} @ {}fps",
            width, height, fps
        );
        let open_result =
            sys::avformat_open_input(&mut fmt_ctx, url.as_ptr(), input_format, dict.as_mut_ptr());
        if open_result < 0 {
            error!(
                "[recorder] Failed to open avfoundation input. Error code: {}. This usually means:",
                open_result
            );
            error!("[recorder] 1. Screen recording permissions not granted");
            error!(
                "[recorder] 2. Wrong device index (try running: ffmpeg -f avfoundation -list_devices true -i \"\")"
            );
            error!("[recorder] 3. Another app is using the capture device");
            error!(
                "[recorder] Fix: Go to System Preferences > Security & Privacy > Privacy > Screen Recording"
            );
            error!("[recorder] and grant permission to your terminal/application, then restart.");
            return;
        }
        info!("[recorder] Successfully opened AVFoundation input");

        if sys::avformat_find_stream_info(fmt_ctx, ptr::null_mut()) < 0 {
            error!(
                "[recorder] Failed to find stream info - the capture device may not be available"
            );
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }
        info!("[recorder] Found {} streams", (*fmt_ctx).nb_streams);

        let mut video_stream_index = -1;
        for i in 0..(*fmt_ctx).nb_streams as i32 {
            let stream = *(*fmt_ctx).streams.add(i as usize);
            if (*(*stream).codecpar).codec_type == sys::AVMediaType::AVMEDIA_TYPE_VIDEO {
                video_stream_index = i;
                break;
            }
        }

        if video_stream_index == -1 {
            error!(
                "[recorder] Failed to find video stream in {} available streams",
                (*fmt_ctx).nb_streams
            );
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }
        info!(
            "[recorder] Found video stream at index {}",
            video_stream_index
        );

        let input_stream = *(*fmt_ctx).streams.add(video_stream_index as usize);
        let input_codecpar = (*input_stream).codecpar;

        let decoder = sys::avcodec_find_decoder((*input_codecpar).codec_id);
        if decoder.is_null() {
            error!(
                "[recorder] Failed to find decoder for codec ID: {:?}",
                (*input_codecpar).codec_id
            );
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        dec_ctx = sys::avcodec_alloc_context3(decoder);
        if dec_ctx.is_null() {
            error!("[recorder] Failed to allocate decoder context");
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        if sys::avcodec_parameters_to_context(dec_ctx, input_codecpar) < 0 {
            error!("[recorder] Failed to copy decoder parameters");
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        if sys::avcodec_open2(dec_ctx, decoder, ptr::null_mut()) < 0 {
            error!("[recorder] Failed to open decoder");
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }
        info!("[recorder] Decoder initialized successfully");

        // Create scaler to convert from input format to YUV420P
        scaler_ctx = sys::sws_getContext(
            (*dec_ctx).width,
            (*dec_ctx).height,
            (*dec_ctx).pix_fmt,
            width as i32,
            height as i32,
            sys::AVPixelFormat::AV_PIX_FMT_YUV420P,
            sys::SWS_BILINEAR as i32,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );

        if scaler_ctx.is_null() {
            error!(
                "[recorder] Failed to create scaler context from {}x{} to {}x{}",
                (*dec_ctx).width,
                (*dec_ctx).height,
                width,
                height
            );
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }
        info!(
            "[recorder] Scaler context created for {}x{} -> {}x{}",
            (*dec_ctx).width,
            (*dec_ctx).height,
            width,
            height
        );

        // Try to find the best available encoder for macOS
        // VideoToolbox is hardware-accelerated and preferred on macOS
        let preferred_encoders = ["h264_videotoolbox", "libx264", "h264"];
        let mut encoder: *const sys::AVCodec = ptr::null();
        let mut chosen_encoder_name = "";

        for name in preferred_encoders.iter() {
            let c_name = cstring!(*name);
            let found_encoder = sys::avcodec_find_encoder_by_name(c_name.as_ptr());
            if !found_encoder.is_null() {
                encoder = found_encoder;
                chosen_encoder_name = *name;
                info!("[recorder] Selected encoder: {}", chosen_encoder_name);
                break;
            }
        }

        if encoder.is_null() {
            error!(
                "[recorder] Could not find any suitable H.264 encoder. Available encoders may be limited."
            );
            error!(
                "[recorder] Make sure FFmpeg is compiled with VideoToolbox support for hardware acceleration."
            );
            // Perform cleanup
            sys::sws_freeContext(scaler_ctx);
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        enc_ctx = sys::avcodec_alloc_context3(encoder);
        if enc_ctx.is_null() {
            error!("[recorder] Failed to allocate encoder context");
            // Perform cleanup
            sys::sws_freeContext(scaler_ctx);
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        (*enc_ctx).width = width as i32;
        (*enc_ctx).height = height as i32;
        (*enc_ctx).pix_fmt = sys::AVPixelFormat::AV_PIX_FMT_YUV420P;
        (*enc_ctx).time_base = sys::AVRational {
            num: 1,
            den: fps as i32,
        };
        (*enc_ctx).framerate = sys::AVRational {
            num: fps as i32,
            den: 1,
        };

        (*enc_ctx).gop_size = fps as i32;
        (*enc_ctx).max_b_frames = 0;

        let mut enc_opts = AVDict::new();

        // Apply encoder-specific settings
        match chosen_encoder_name {
            "h264_videotoolbox" => {
                info!("[recorder] Applying VideoToolbox settings");
                enc_opts.set("profile", "main");
                enc_opts.set("level", "4.0");
                enc_opts.set("crf", "23"); // Constant rate factor for quality
                enc_opts.set("realtime", "1"); // Enable real-time encoding
            }
            "libx264" => {
                info!("[recorder] Applying libx264 settings");
                enc_opts.set("preset", "veryfast");
                enc_opts.set("tune", "zerolatency");
                enc_opts.set("crf", "22");
            }
            _ => {}
        }

        if sys::avcodec_open2(enc_ctx, encoder, enc_opts.as_mut_ptr()) < 0 {
            error!(
                "[recorder] Failed to open {} encoder. Trying with default settings...",
                chosen_encoder_name
            );

            // Try again with minimal settings
            let mut fallback_opts = AVDict::new();
            if sys::avcodec_open2(enc_ctx, encoder, fallback_opts.as_mut_ptr()) < 0 {
                error!("[recorder] Failed to open encoder even with fallback settings");
                sys::sws_freeContext(scaler_ctx);
                sys::avcodec_free_context(&mut dec_ctx);
                sys::avcodec_free_context(&mut enc_ctx);
                sys::avformat_close_input(&mut fmt_ctx);
                return;
            }
            info!("[recorder] Encoder opened with fallback settings");
        } else {
            info!("[recorder] Encoder opened successfully with optimized settings");
        }

        // Store codec parameters for later use in save()
        {
            let encoder_codecpar = sys::avcodec_parameters_alloc();
            if sys::avcodec_parameters_from_context(encoder_codecpar, enc_ctx) >= 0 {
                let mut lock = codecpar_arc.lock().unwrap();
                *lock = Some(CodecParPtr(encoder_codecpar));
            }
        }

        // Allocate frames and packets
        packet = sys::av_packet_alloc();
        decoded_frame = sys::av_frame_alloc();
        scaled_frame = sys::av_frame_alloc();

        if packet.is_null() || decoded_frame.is_null() || scaled_frame.is_null() {
            error!("[recorder] Failed to allocate packet or frames");
            if !packet.is_null() {
                sys::av_packet_free(&mut packet);
            }
            if !decoded_frame.is_null() {
                sys::av_frame_free(&mut decoded_frame);
            }
            if !scaled_frame.is_null() {
                sys::av_frame_free(&mut scaled_frame);
            }
            sys::sws_freeContext(scaler_ctx);
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avcodec_free_context(&mut enc_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        (*scaled_frame).width = width as i32;
        (*scaled_frame).height = height as i32;
        (*scaled_frame).format = sys::AVPixelFormat::AV_PIX_FMT_YUV420P as i32;

        if sys::av_frame_get_buffer(scaled_frame, 0) < 0 {
            error!("[recorder] Failed to allocate frame buffer");
            sys::av_packet_free(&mut packet);
            sys::av_frame_free(&mut decoded_frame);
            sys::av_frame_free(&mut scaled_frame);
            sys::sws_freeContext(scaler_ctx);
            sys::avcodec_free_context(&mut dec_ctx);
            sys::avcodec_free_context(&mut enc_ctx);
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        let mut frame_index = 0i64;

        info!("[recorder] Starting macOS screen capture loop");
        let mut total_frames = 0u64;
        let mut encoded_frames = 0u64;

        // Main capture loop
        while sys::av_read_frame(fmt_ctx, packet) >= 0 {
            total_frames += 1;
            if *stop_signal.blocking_lock() {
                sys::av_packet_unref(packet);
                break;
            }

            if (*packet).stream_index == video_stream_index {
                if sys::avcodec_send_packet(dec_ctx, packet) >= 0 {
                    while sys::avcodec_receive_frame(dec_ctx, decoded_frame) >= 0 {
                        // Scale the frame from input format to YUV420P
                        sys::sws_scale(
                            scaler_ctx,
                            (*decoded_frame).data.as_ptr() as *const *const u8,
                            (*decoded_frame).linesize.as_ptr(),
                            0,
                            (*dec_ctx).height,
                            (*scaled_frame).data.as_ptr(),
                            (*scaled_frame).linesize.as_ptr(),
                        );

                        (*scaled_frame).pts = frame_index;
                        frame_index += 1;

                        // Encode the scaled frame
                        if sys::avcodec_send_frame(enc_ctx, scaled_frame) >= 0 {
                            loop {
                                let mut enc_packet = sys::av_packet_alloc();
                                let ret = sys::avcodec_receive_packet(enc_ctx, enc_packet);
                                if ret == sys::AVERROR(sys::EAGAIN) || ret == sys::AVERROR_EOF {
                                    sys::av_packet_free(&mut enc_packet);
                                    break;
                                } else if ret < 0 {
                                    error!("[recorder] Error receiving packet from encoder");
                                    sys::av_packet_free(&mut enc_packet);
                                    break;
                                }

                                let is_key =
                                    ((*enc_packet).flags & sys::AV_PKT_FLAG_KEY as i32) != 0;
                                let data = std::slice::from_raw_parts(
                                    (*enc_packet).data,
                                    (*enc_packet).size as usize,
                                );
                                replay_buffer.add_packet(data.to_vec(), is_key);
                                encoded_frames += 1;

                                if encoded_frames % (fps as u64 * 5) == 0 {
                                    info!(
                                        "[recorder] Encoded {} frames ({}% keyframes)",
                                        encoded_frames,
                                        if encoded_frames > 0 {
                                            (encoded_frames * 100) / total_frames
                                        } else {
                                            0
                                        }
                                    );
                                }

                                sys::av_packet_unref(enc_packet);
                                sys::av_packet_free(&mut enc_packet);
                            }
                        }
                    }
                }
            }
            sys::av_packet_unref(packet);
        }

        // Cleanup resources
        sys::av_frame_free(&mut decoded_frame);
        sys::av_frame_free(&mut scaled_frame);
        sys::av_packet_free(&mut packet);

        sys::sws_freeContext(scaler_ctx);

        sys::avcodec_free_context(&mut dec_ctx);
        sys::avcodec_free_context(&mut enc_ctx);

        sys::avformat_close_input(&mut fmt_ctx);
        info!(
            "[recorder] macOS capture thread stopped. Total frames: {}, Encoded frames: {}",
            total_frames, encoded_frames
        );
    }
}
