
use std::{
    io,
    io::Write,
    time::Instant,
    collections::VecDeque,
    fs::File,
};
use std::io::ErrorKind::WouldBlock;
use scrap::{Display, Capturer, Frame as s_Frame};
use reqwest::blocking::Client;
use std::io::Read;
use std::ops::Deref;
use std::time::Duration;
use anyhow::Result;
use device_query::{DeviceQuery, DeviceState, Keycode};
use lazy_static::lazy_static;
use lz4::{EncoderBuilder, Decoder};
use windows::Foundation::TimeSpan;
use serde_json::Value;
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
    encoder::{AudioSettingBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder}
};

pub struct CompressedFrame {
    pub compressed_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub time: TimeSpan,
}

lazy_static! {
    static ref DEVICE_STATE: DeviceState = DeviceState::new();
}

struct Capture {
    frame_buffer: VecDeque<CompressedFrame>,
    start: Instant,
    fps: u32,
}

impl Capture {
    fn new() -> Result<Self> {
        Ok(Self {
            frame_buffer: VecDeque::with_capacity(60 * 10),
            start: Instant::now(),
            fps: 60,
        })
    }

    fn start_scrap_cap(&mut self, capturer: &mut Capturer) -> Result<()> {
        loop {
            match capturer.frame() {
                Ok(frame) => {
                    let elapsed_time = self.start.elapsed();
                    let frame_index = self.frame_buffer.len() as u32;
                    let frame_time = TimeSpan::from(Duration::from_micros((frame_index * 1_000_000 / self.fps) as u64));
                    let compressed = Self::compress_frame(frame.deref())?;
                    let frame_data = CompressedFrame {
                        compressed_data: compressed,
                        width: capturer.width() as u32,
                        height: capturer.height() as u32,
                        time: frame_time
                    };

                    self.frame_buffer.push_back(frame_data);

                    let buffer_size = self.fps * 10;
                    while self.frame_buffer.len() > buffer_size as usize {
                        drop(self.frame_buffer.pop_front().unwrap());
                    }

                    print!("\rRecording for: {} seconds", elapsed_time.as_secs());
                    io::stdout().flush()?;

                    if DEVICE_STATE.get_keys().contains(&Keycode::Key9) {
                        println!("\nSaving buffer to file...");
                        self.save_buffer()?;
                        return Ok(());
                    }
                }
                Err(ref e) if e.kind() == WouldBlock => {
                    // Wait for the next frame
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => return Err(e.into()),
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
        let mut encoder  = EncoderBuilder::new().level(1).build(Vec::new())?;
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
            VideoSettingsBuilder::new(self.frame_buffer[0].width, self.frame_buffer[0].height).frame_rate(self.fps),
            AudioSettingBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            "output.mp4"
        )?;

        for frame in self.frame_buffer.iter() {
            let mut decompressed = Self::decompress_frame(&frame.compressed_data)?;
            Self::convert_to_bottom_up(&mut decompressed, frame.width, frame.height);
            encoder.send_frame_buffer(&decompressed, frame.time.Duration);
        
    }
    encoder.finish()?;
    Ok(())
}

    fn upload_file(file_path: &str) -> Result<()> {
        let client = Client::new();
        let file = File::open(file_path)?;

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
    let mut capture = Capture::new()?;

    capture.start_scrap_cap(&mut capturer)
}