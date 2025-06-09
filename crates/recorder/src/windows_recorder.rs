use async_trait::async_trait;
use log::{debug, error, info};
use std::{ffi::CString, ptr, sync::Arc};
use tokio::sync::Mutex;
use crate::cstring;
use ffmpeg_next::software::scaling;
use crate::avdict::AVDict;

pub use ffmpeg_next::sys;

use super::recorder::Recorder;
use storage::ReplayBuffer;

type ArcM<T> = Arc<Mutex<T>>;

#[derive(Copy, Clone)]
pub struct CodecParPtr(pub *mut sys::AVCodecParameters);
unsafe impl Send for CodecParPtr {}
unsafe impl Sync for CodecParPtr {}

pub struct WindowsRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    stop_signal: ArcM<bool>,
    replay_buffer: Arc<ReplayBuffer>,
    codecpar: std::sync::Arc<std::sync::Mutex<Option<CodecParPtr>>>,
}

// SAFETY: We ensure the pointer is only used while valid.
unsafe impl Send for WindowsRecorder {}
unsafe impl Sync for WindowsRecorder {}



#[async_trait]
impl Recorder for WindowsRecorder {
    fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, _output: String) -> Self {
        ffmpeg_next::init().expect("FFmpeg init failed");
        unsafe { sys::avdevice_register_all() };

        let estimated_packets = (fps as usize) * (buffer_secs as usize) * 2;
        let replay_buffer = Arc::new(ReplayBuffer::new(buffer_secs, estimated_packets));

        Self {
            width,
            height,
            fps,
            buffer_secs,
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

        tokio::task::spawn_blocking(move || {
            capture_encode_loop_sys(width, height, fps, secs, buf, stop, codecpar_arc);
        });

        info!("[recorder] ffmpeg-sys capture thread started");
    }

    async fn stop(&mut self) {
        *self.stop_signal.lock().await = true;
    }

    fn save(&self, final_output_path: &str) -> Result<(), String> {
        let codecpar = self.codecpar.lock().unwrap();
        let codecpar = codecpar.ok_or("Codec parameters not set")?.0;
        info!("[recorder] Saving replay buffer to {}", final_output_path);
        self.replay_buffer.save_to_file(final_output_path, codecpar)
    }

    fn get_output_path(&self) -> &str {
        "output.mp4"
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

        let input_format = sys::av_find_input_format(cstring!("gdigrab").as_ptr());
        if input_format.is_null() {
            error!("[recorder] Failed to find gdigrab input format");
            return;
        }

        let mut dict = AVDict::new();
        dict.set("framerate", &fps.to_string());
        dict.set("video_size", &format!("{}x{}", width, height));

        let url = CString::new("desktop").unwrap();
        let ret =
            sys::avformat_open_input(&mut fmt_ctx, url.as_ptr(), input_format, dict.as_mut_ptr());
        if ret < 0 {
            return;
        }

        if sys::avformat_find_stream_info(fmt_ctx, ptr::null_mut()) < 0 {
            error!("[recorder] Failed to find stream info");
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        let mut video_stream_index = -1;
        for i in 0..(*fmt_ctx).nb_streams as i32 {
            let stream = *(*fmt_ctx).streams.add(i as usize);
            if (*(*stream).codecpar).codec_type == sys::AVMediaType::AVMEDIA_TYPE_VIDEO {
                video_stream_index = i;
                break;
            }
        }

        if video_stream_index == -1 {
            error!("[recorder] Failed to find video stream");
            sys::avformat_close_input(&mut fmt_ctx);
            return;
        }

        let input_stream = *(*fmt_ctx).streams.add(video_stream_index as usize);
        let input_codecpar = (*input_stream).codecpar;

        let decoder = sys::avcodec_find_decoder((*input_codecpar).codec_id);
        dec_ctx = sys::avcodec_alloc_context3(decoder);
        sys::avcodec_parameters_to_context(dec_ctx, input_codecpar);
        sys::avcodec_open2(dec_ctx, decoder, ptr::null_mut());

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

        let encoder = sys::avcodec_find_encoder(sys::AVCodecID::AV_CODEC_ID_H264);
        enc_ctx = sys::avcodec_alloc_context3(encoder);
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
        enc_opts.set("preset", "fast");  // Better quality than ultrafast
        enc_opts.set("crf", "18");       // High quality (lower is better, 18 is very high quality)
        enc_opts.set("tune", "zerolatency");
        sys::avcodec_open2(enc_ctx, encoder, enc_opts.as_mut_ptr());

        // Store encoder's codec parameters for later use
        {
            let encoder_codecpar = sys::avcodec_parameters_alloc();
            if sys::avcodec_parameters_from_context(encoder_codecpar, enc_ctx) >= 0 {
                let mut lock = codecpar_arc.lock().unwrap();
                *lock = Some(CodecParPtr(encoder_codecpar));
            }
        }

        // Where the real work happens
        packet = sys::av_packet_alloc();
        decoded_frame = sys::av_frame_alloc();
        scaled_frame = sys::av_frame_alloc();
        (*scaled_frame).width = width as i32;
        (*scaled_frame).height = height as i32;
        (*scaled_frame).format = sys::AVPixelFormat::AV_PIX_FMT_YUV420P as i32;
        sys::av_frame_get_buffer(scaled_frame, 0);

        let mut frame_index = 0;

        while sys::av_read_frame(fmt_ctx, packet) >= 0 {
            if *stop_signal.blocking_lock() {
                sys::av_packet_unref(packet);
                break;
            }

            if (*packet).stream_index == video_stream_index {
                if sys::avcodec_send_packet(dec_ctx, packet) >= 0 {
                    while sys::avcodec_receive_frame(dec_ctx, decoded_frame) >= 0 {
                        //Scale the frame
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
                            let mut enc_packet = sys::av_packet_alloc();
                            while sys::avcodec_receive_packet(enc_ctx, enc_packet) >= 0 {
                                let is_key =
                                    ((*enc_packet).flags & sys::AV_PKT_FLAG_KEY as i32) != 0;
                                let data = std::slice::from_raw_parts(
                                    (*enc_packet).data,
                                    (*enc_packet).size as usize,
                                );
                                replay_buffer.add_packet(data.to_vec(), is_key);
                                sys::av_packet_unref(enc_packet);
                            }
                            sys::av_packet_free(&mut enc_packet);
                        }
                    }
                }
            }
            sys::av_packet_unref(packet);
        }

        sys::av_frame_free(&mut decoded_frame);
        sys::av_frame_free(&mut scaled_frame);
        sys::av_packet_free(&mut packet);

        sys::sws_freeContext(scaler_ctx);

        sys::avcodec_free_context(&mut dec_ctx);
        sys::avcodec_free_context(&mut enc_ctx);

        sys::avformat_close_input(&mut fmt_ctx);
    }
}