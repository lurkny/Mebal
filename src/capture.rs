use std::{
    collections::VecDeque,
    io::{self, ErrorKind::WouldBlock, Write},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use scrap::Capturer;

use crate::compression::{CompressedFrame, compress_frame};
use crate::video_encoding::save_buffer;

pub struct Capture {
    frame_buffer: VecDeque<CompressedFrame>,
    start: Instant,
    fps: u32,
    stop_flag: Arc<AtomicBool>,
}

impl Capture {
    pub fn new(fps: u32, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        Ok(Self {
            frame_buffer: VecDeque::with_capacity(fps as usize * 10),
            start: Instant::now(),
            fps,
            stop_flag,
        })
    }

    pub fn start_scrap_cap(&mut self, capturer: &mut Capturer) -> Result<()> {
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps as f64);
        let mut next_capture_time = self.start;

        loop {
            if self.stop_flag.load(Ordering::SeqCst) {
                println!("\nSaving buffer to file...");
                save_buffer(&self.frame_buffer, self.fps)?;
                return Ok(());
            }

            let now = Instant::now();
            if now >= next_capture_time {
                match capturer.frame() {
                    Ok(frame) => {
                        let elapsed_time = now.duration_since(self.start);
                        let compressed = compress_frame(frame.deref())?;
                        let frame_data = CompressedFrame {
                            compressed_data: compressed,
                            width: capturer.width() as u32,
                            height: capturer.height() as u32,
                            timestamp: elapsed_time,
                        };

                        self.frame_buffer.push_back(frame_data);

                        print!("\rRecording for: {:.2} seconds", elapsed_time.as_secs_f32());
                        io::stdout().flush()?;

                        next_capture_time += frame_duration;
                    }
                    Err(ref e) if e.kind() == WouldBlock => {
                        // Frame not ready, try again on next iteration
                    }
                    Err(e) => return Err(e.into()),
                }
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}