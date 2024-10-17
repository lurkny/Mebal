use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use anyhow::Result;
use scrap::{Capturer, Display};
use crate::{compression::{compress_frame, decompress_frame, CompressedFrame}, ffmpeg_encoder::FFmpegEncoder, video_encoding_strategy::VideoEncodingStrategy};
use std::fs::File;
use std::io::Write;

pub struct GenericCapture {
    frame_buffer: VecDeque<CompressedFrame>,
    start: Instant,
    fps: u32,
    stop_flag: Arc<AtomicBool>,
}

impl GenericCapture {
    const MAX_FRAME_BUFFER_SIZE: usize = (60 * 10) as usize;

    pub fn new(fps: u32, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        Ok(Self {
            frame_buffer: VecDeque::with_capacity(fps as usize * 10),
            start: Instant::now(),
            fps,
            stop_flag,
        })
    }

    pub fn start_capture(&mut self) -> Result<()> {
        let display = Display::primary()?;
        let mut capturer = Capturer::new(display)?;
        let cap_width = capturer.width();
        let cap_height = capturer.height();
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps as f64);
        let mut next_capture_time = self.start;

        loop {
            if self.stop_flag.load(Ordering::SeqCst) || self.frame_buffer.len() >= Self::MAX_FRAME_BUFFER_SIZE {
                println!("\nSaving buffer to file...");
                let mut encoder = VideoEncodingStrategy::new(self.frame_buffer[0].width, self.frame_buffer[0].height, self.fps, "output.mp4")?;
                for frame in &self.frame_buffer {
                    encoder.encode_frame(frame)?;
                }
                encoder.finish()?;
                return Ok(());
            }

            let now = Instant::now();
            if now >= next_capture_time {
                match capturer.frame() {
                    Ok(frame) => {
                        let elapsed_time = now.duration_since(self.start);
                        self.process_frame(&frame, &cap_width, &cap_height, elapsed_time)?;
                        println!("\rRecording for: {:.2} seconds", elapsed_time.as_secs_f32());
                    }
                    Err(error) => {
                        if error.kind() == std::io::ErrorKind::WouldBlock {
                            // Frame not ready yet, try again later
                            std::thread::sleep(Duration::from_millis(1));
                            continue;
                        } else {
                            return Err(error.into());
                        }
                    }
                }
                next_capture_time += frame_duration;
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    fn process_frame(&mut self, frame: &[u8], width: &usize, height: &usize, timestamp: Duration) -> Result<()> {
        let compressed = compress_frame(frame)?;
        let frame_data = CompressedFrame {
            compressed_data: compressed,
            width: *width as u32,
            height: *height as u32,
            timestamp,
        };

        // Save the decompressed frame data to disk before encoding with FFmpeg
        let decompressed = decompress_frame(&frame_data.compressed_data)?;

        // Write raw decompressed frame to disk
        let mut file = File::options().append(true).create(true).open("raw_video.raw")?;
        file.write_all(&decompressed)?;
        if Self::MAX_FRAME_BUFFER_SIZE <= self.frame_buffer.len() {
            self.frame_buffer.pop_front();
        }

        self.frame_buffer.push_back(frame_data);
        Ok(())
    }
}