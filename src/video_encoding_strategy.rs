use std::collections::VecDeque;
use anyhow::Result;
use crate::compression::CompressedFrame;

#[cfg(target_os = "windows")]
use crate::windows_encoder::WindowsEncoder;

#[cfg(not(target_os = "windows"))]
use crate::ffmpeg_encoder::FFmpegEncoder;

pub enum VideoEncodingStrategy {
    #[cfg(target_os = "windows")]
    Windows(WindowsEncoder),
    #[cfg(not(target_os = "windows"))]
    FFmpeg(FFmpegEncoder),
}

impl VideoEncodingStrategy {
    pub fn new(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            Ok(VideoEncodingStrategy::Windows(WindowsEncoder::new(width, height, fps, output_path)?))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(VideoEncodingStrategy::FFmpeg(FFmpegEncoder::new(width, height, fps, output_path)?))
        }
    }

    pub fn encode_frame(&mut self, frame: &CompressedFrame) -> Result<()> {
        match self {
            #[cfg(target_os = "windows")]
            VideoEncodingStrategy::Windows(encoder) => encoder.encode_frame(frame),
            #[cfg(not(target_os = "windows"))]
            VideoEncodingStrategy::FFmpeg(encoder) => encoder.encode_frame(&frame.compressed_data),
        }
    }

    pub fn finish(self) -> Result<()> {
        match self {
            #[cfg(target_os = "windows")]
            VideoEncodingStrategy::Windows(encoder) => encoder.finish(),
            #[cfg(not(target_os = "windows"))]
            VideoEncodingStrategy::FFmpeg(encoder) => encoder.finish(),
        }
    }
}

pub fn save_buffer(frame_buffer: &VecDeque<CompressedFrame>, fps: u32) -> Result<()> {
    if let Some(first_frame) = frame_buffer.front() {
        let mut encoder = VideoEncodingStrategy::new(first_frame.width, first_frame.height, fps, "output.mp4")?;

        for frame in frame_buffer {
            encoder.encode_frame(frame)?;
        }

        encoder.finish()?;
    }
    Ok(())
}