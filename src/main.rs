use std::{
    collections::VecDeque,
    io::{self, ErrorKind::WouldBlock, Read, Write},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use lz4::{Decoder, EncoderBuilder};
use rdev::{listen, EventType, Key};
use reqwest::blocking::Client;
use scrap::{Capturer, Display};
use serde_json::Value;
use windows::Foundation::TimeSpan;
use windows_capture::encoder::{
    AudioSettingBuilder,
    ContainerSettingsBuilder,
    VideoEncoder,
    VideoSettingsBuilder,
};

pub struct CompressedFrame {
    pub compressed_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: Duration,
}



struct Capture {
    frame_buffer: VecDeque<CompressedFrame>,
    start: Instant,
    fps: u32,
    stop_flag: Arc<AtomicBool>,
}

impl Capture {
    fn new(fps: u32, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        Ok(Self {
            frame_buffer: VecDeque::with_capacity(fps as usize * 10),
            start: Instant::now(),
            fps,
            stop_flag,
        })
    }

    fn start_scrap_cap(&mut self, capturer: &mut Capturer) -> Result<()> {
        let frame_duration = Duration::from_secs_f64(1.0 / self.fps as f64);
        let mut next_capture_time = self.start;

        loop {
            if self.stop_flag.load(Ordering::SeqCst) {
                println!("\nSaving buffer to file...");
                self.save_buffer()?;
                return Ok(());
            }

            let now = Instant::now();
            if now >= next_capture_time {
                match capturer.frame() {
                    Ok(frame) => {
                        let elapsed_time = now.duration_since(self.start);
                        let compressed = Self::compress_frame(frame.deref())?;
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

    fn convert_to_bottom_up(buffer: &mut [u8], width: u32, height: u32) {
        let stride = width as usize * 4;
        let half_height = height as usize / 2;
        for y in 0..half_height {
            let top = y * stride;
            let bottom = (height as usize - 1 - y) * stride;
            let (top_slice, bottom_slice) = buffer.split_at_mut(bottom);
            top_slice[top..top + stride].swap_with_slice(&mut bottom_slice[..stride]);
        }
    }

    fn compress_frame(buffer: &[u8]) -> Result<Vec<u8>> {
        let mut encoder  = EncoderBuilder::new().level(0).favor_dec_speed(true).build(Vec::new())?;
        encoder.write_all(buffer)?;
        let (compressed_data, result) = encoder.finish();
        result.map_err(|e| e.into()).map(|_| compressed_data)
    }

    fn decompress_frame(compressed_data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = Decoder::new(compressed_data)?;
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;
        Ok(decompressed_data)
    }

    fn save_buffer(&self) -> Result<()> {
        let mut encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(self.frame_buffer[0].width, self.frame_buffer[0].height)
                .frame_rate(self.fps)
                .bitrate(5_000_000),  // 5 Mbps, adjust as needed
            AudioSettingBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            "output.mp4"
        )?;

        for frame in self.frame_buffer.iter() {
            let mut decompressed = Self::decompress_frame(&frame.compressed_data)?;
            Self::convert_to_bottom_up(&mut decompressed, frame.width, frame.height);
            let frame_time = TimeSpan::from(frame.timestamp);
            encoder.send_frame_buffer(&decompressed, frame_time.Duration)?;
        }
        
        encoder.finish()?;
        Ok(())
    }

    #[allow(unused)]
    fn upload_file(file_path: &str) -> Result<()> {
        let client = Client::new();

        let upload_url_response = client.post("https://videolink.brodymlarson2.workers.dev/upload")
            .header("x-secret-password", "pls-no-dos")
            .send()?;

        if !upload_url_response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get upload URL: {}", upload_url_response.status()));
        }

        let json: Value = upload_url_response.json()?;
        let upload_url = json["result"]["uploadURL"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract uploadURL from response"))?;

        println!("Got upload URL: {}", upload_url);

        let form = reqwest::blocking::multipart::Form::new()
            .file("file", file_path)?;

        let response = client.post(upload_url)
            .multipart(form)
            .send()?;

        if response.status().is_success() {
            println!("File uploaded successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Upload failed with status: {}", response.status()))
        }
    }
}

fn main() -> Result<()> {
    let display = Display::primary()?;
    let mut capturer = Capturer::new(display)?;
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    // Set up global hotkey listener
    thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            if let EventType::KeyPress(Key::F2) = event.event_type {
                println!("F2 pressed, stopping capture...");
                stop_flag_clone.store(true, Ordering::SeqCst);
            }
        }) {
            println!("Error setting up hotkey listener: {:?}", error);
        }
    });

    let mut capture = Capture::new(60, stop_flag)?;  // Specify desired FPS here
    capture.start_scrap_cap(&mut capturer)
}
