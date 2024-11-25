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
use crate::{compression::{compress_frame, CompressedFrame}, video_encoding_strategy::VideoEncodingStrategy};

pub struct GenericCapture {
    frame_buffer: VecDeque<CompressedFrame>,
    fps: u32,
    stop_flag: Arc<AtomicBool>,
    output_path: String,
    frame_count: usize,
}

impl GenericCapture {
    const MAX_BUFFER_DURATION_SECS: u32 = 60 * 10;

    pub fn new(fps: u32, stop_flag: Arc<AtomicBool>, output_path: String) -> Result<Self> {
        let buffer_capacity = (fps as usize) * Self::MAX_BUFFER_DURATION_SECS as usize;
        Ok(Self {
            frame_buffer: VecDeque::with_capacity(buffer_capacity),
            fps,
            stop_flag,
            output_path,
            frame_count: 0,
        })
    }

    pub fn start_capture(&mut self) -> Result<()> {
        let display = Display::primary()?;
        let mut capturer = Capturer::new(display)?;
        let frame_duration = Duration::from_secs_f64(1.0 / f64::from(self.fps));
        let capture_start = Instant::now();

        println!("Starting capture at {} FPS...", self.fps);

        while !self.stop_flag.load(Ordering::SeqCst) {
            let frame_start = Instant::now();

            if let Err(e) = self.handle_frame_capture(&mut capturer) {
                eprintln!("Frame capture error: {}", e);
            }

            self.frame_count += 1;

            if self.frame_count % 60 == 0 {
                println!("Captured {} frames in {:.2}s", 
                         self.frame_count,
                         capture_start.elapsed().as_secs_f32());
            }

            if let Some(sleep_time) = frame_duration.checked_sub(frame_start.elapsed()) {
                std::thread::sleep(sleep_time);
            }
        }

        println!("Capture stopped after {:.2}s with {} frames", 
                 capture_start.elapsed().as_secs_f32(),
                 self.frame_count);

        self.save_buffer_to_file()
    }

    fn handle_frame_capture(&mut self, capturer: &mut Capturer) -> Result<()> {
        match capturer.frame() {
            Ok(frame) => {
                let compressed = compress_frame(&frame)?;
                let frame_data = CompressedFrame {
                    compressed_data: compressed,
                    width: capturer.width() as u32,
                    height: capturer.height() as u32,
                    timestamp: Duration::from_secs_f64(self.frame_count as f64 / self.fps as f64),
                };
                self.frame_buffer.push_back(frame_data);
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
                Ok(())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn save_buffer_to_file(&self) -> Result<()> {
        println!("\nSaving buffer to file...");
        if self.frame_buffer.is_empty() {
            return Ok(());
        }

        let first_frame = &self.frame_buffer[0];
        let mut encoder = VideoEncodingStrategy::new(
            first_frame.width,
            first_frame.height,
            self.fps,
            &self.output_path,
        )?;

        for frame in &self.frame_buffer {
            encoder.encode_frame(frame)?;
        }
        encoder.finish()
    }
}