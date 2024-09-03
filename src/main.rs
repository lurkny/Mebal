use std::{
    io,
    io::Write,
    time::Instant,
    collections::VecDeque,
    fs::File,
};
use reqwest::{
    blocking::Client,
};
use std::io::Read;
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
    encoder::{AudioSettingBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder}
};
use anyhow::{Result,Context};
use device_query::{DeviceQuery, DeviceState, Keycode};
use lazy_static::lazy_static;
use lz4::{EncoderBuilder, Decoder};
use windows::Foundation::TimeSpan;
use serde_json::Value;

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
    encoder: Option<VideoEncoder>,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = String;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(message: Self::Flags) -> Result<Self, Self::Error> {
        println!("Got The Flag: {message}");

        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(2560,1440 ).frame_rate(60),
            AudioSettingBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            "output.mp4"
        )?;

        Ok(Self {
            frame_buffer: VecDeque::with_capacity(60 * 10),
            start: Instant::now(),
            fps: 60,
            encoder: Some(encoder)
        })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, capture_control: InternalCaptureControl) -> Result<(), Self::Error> {
        print!("\rRecording for: {} seconds", self.start.elapsed().as_secs());
        io::stdout().flush()?;

        let mut raw_buffer = frame.buffer().unwrap();
        let compressed = Self::compress_frame(&raw_buffer.as_raw_nopadding_buffer().unwrap())?;

        let frame_data = CompressedFrame {
            compressed_data: compressed,
            width: frame.width(),
            height: frame.height(),
            time: frame.timespan()
        };

        self.frame_buffer.push_back(frame_data);

        let buffer_size = self.fps * 10;
        while self.frame_buffer.len() > buffer_size as usize {
            drop(self.frame_buffer.pop_front().unwrap());
        }

        if DEVICE_STATE.get_keys().contains(&Keycode::Key9) {
            println!("\nSaving buffer to file...");
            let output_path = format!("output_{}.mp4", self.start.elapsed().as_secs());
            let mut encoder = VideoEncoder::new(
                VideoSettingsBuilder::new(self.frame_buffer[0].width, self.frame_buffer[0].height).frame_rate(self.fps),
                AudioSettingBuilder::default().disabled(true),
                ContainerSettingsBuilder::default(),
                output_path.clone()
            )?;

            for compressed_frame in self.frame_buffer.iter() {
                let mut decompressed_data = Self::decompress_frame(&compressed_frame.compressed_data)?;
                Self::convert_to_bottom_up(&mut decompressed_data, compressed_frame.width, compressed_frame.height);
                encoder.send_frame_buffer(&decompressed_data, compressed_frame.time.Duration)?;
            }
            encoder.finish()?;
            println!("Buffer saved successfully.");
            Self::upload_file(&output_path.as_str())?;
        }

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        println!("Capture Session Closed");
        Ok(())
    }
}

impl Capture {
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

    fn upload_file(file_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        let file = File::open(file_path)?;

        // First, get the upload URL
        let upload_url_response = client.post("https://videolink.brodymlarson2.workers.dev/upload")
            .header("x-secret-password", "pls-no-dos")
            .send()?;

        if !upload_url_response.status().is_success() {
            return Err(format!("Failed to get upload URL: {}", upload_url_response.status()).into());
        }

        let json: Value = upload_url_response.json::<Value>()?;
        let upload_url = json["result"]["uploadURL"].as_str()
            .ok_or("Failed to extract uploadURL from response")?;

        println!("Got upload URL: {}", upload_url);

        // Now, upload the file to the obtained URL
        let form = reqwest::blocking::multipart::Form::new()
            .file("file", file_path)?;

        let response = client.post(upload_url)
            .multipart(form)
            .send()?;

        if response.status().is_success() {
            println!("File uploaded successfully");
            Ok(())
        } else {
            Err(format!("Upload failed with status: {}", response.status()).into())
        }
    }
}

fn main() {
    let primary_monitor = Monitor::primary().expect("There is no primary monitor");
    let settings = Settings::new(
        primary_monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        ColorFormat::Bgra8,
        "Yea This Works".to_string(),
    );

    Capture::start(settings).expect("Screen Capture Failed");
}