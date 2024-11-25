use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use win_desktop_duplication::{ DesktopDuplicationApi, tex_reader::TextureReader, devices::*, set_process_dpi_awareness, co_init };
use win_desktop_duplication::{tex_reader::*, devices::*};
use anyhow::Result;
use crate::video_encoding_strategy::save_buffer;
use crate::compression::{CompressedFrame, compress_frame};

pub struct WindowsCapture {
    frame_buffer: VecDeque<CompressedFrame>,
    start: Instant,
    fps: u32,
    stop_flag: Arc<AtomicBool>,
}

impl WindowsCapture {
    const MAX_FRAME_BUFFER_SIZE: usize = 60 * 10;

    pub fn new(fps: u32, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        Ok(Self {
            frame_buffer: VecDeque::with_capacity(Self::MAX_FRAME_BUFFER_SIZE),
            start: Instant::now(),
            fps,
            stop_flag,
        })
    }

    pub fn start_duplication_cap(&mut self) -> Result<()> {
        // This is required to use the Desktop Duplication API
        set_process_dpi_awareness();
        co_init();

        // Select the GPU and output to capture
        let adapter = AdapterFactory::new().get_adapter_by_idx(0).unwrap();
        let output = adapter.get_display_by_idx(0).unwrap();

        // Get the output duplication API
        let mut dupl = DesktopDuplicationApi::new(adapter, output.clone()).unwrap();

        // Get the device and context for texture reading
        let (device, ctx) = dupl.get_device_and_ctx();
        let mut texture_reader = TextureReader::new(device, ctx);

        // Create a buffer for storing image data
        let mut pic_data: Vec<u8> = vec![0; 0];

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
                output.wait_for_vsync().unwrap();
                let tex = dupl.acquire_next_frame_now();

                if let Ok(tex) = tex {
                    texture_reader.get_data(&mut pic_data, &tex);
                    let display_mode = output.get_current_display_mode().unwrap();

                    let elapsed_time = now.duration_since(self.start);
                    self.process_frame(&pic_data, display_mode.width, display_mode.height, elapsed_time)?;

                    print!("\rRecording for: {:.2} seconds", elapsed_time.as_secs_f32());
                    io::stdout().flush()?;

                    next_capture_time += frame_duration;
                }
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    fn process_frame(&mut self, pic_data: &[u8], width: u32, height: u32, timestamp: Duration) -> Result<()> {
        let compressed = compress_frame(pic_data)?;
        let frame_data = CompressedFrame {
            compressed_data: compressed,
            width,
            height,
            timestamp,
        };
         if Self::MAX_FRAME_BUFFER_SIZE <= self.frame_buffer.len() {
            self.frame_buffer.pop_front();
        }
        self.frame_buffer.push_back(frame_data);
        Ok(())
    }
}
