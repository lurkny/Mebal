use std::{
    io::Read,
    process::{Command, Stdio, Child},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
use anyhow::{Result, Context};
use crate::utils::storage::FrameBuffer;

pub struct WindowsCapture {
    frame_buffer: Arc<Mutex<FrameBuffer>>,
    ffmpeg_process: Child,
    stop_flag: Arc<AtomicBool>,
}

impl WindowsCapture {
    const BUFFER_SIZE: usize = 600;

    pub fn new(stop_flag: Arc<AtomicBool>, width: u32, height: u32, fps: u32) -> Result<Self> {
        // Initialize FFmpeg command with desktop duplication and audio capture
        let ffmpeg_process = Command::new("ffmpeg")
            .args(&[
                "-y",
                "-f", "dshow",
                "-i", "audio=Microphone",
                "-f", "ddagrab",
                "-framerate", &fps.to_string(),
                "-video_size", &format!("{}x{}", width, height),
                "-i", "desktop",
                "-c:v", "h264_nvenc",
                "-preset", "ultrafast",
                "-c:a", "aac",
                "-b:a", "128k",
                "-pix_fmt", "yuv420p",
                "-f", "mp4",
                "-movflags", "+frag_keyframe+empty_moov+default_base_moof",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn FFmpeg process")?;

        // Initialize FrameBuffer with specified size
        let frame_buffer = Arc::new(Mutex::new(FrameBuffer::new(width, height, Self::BUFFER_SIZE)));

        Ok(Self {
            frame_buffer,
            ffmpeg_process,
            stop_flag,
        })
    }

    pub fn start_capture(&mut self) -> Result<()> {
        let mut stdout = self.ffmpeg_process.stdout.take()
            .context("Failed to capture FFmpeg stdout")?;
        let frame_buffer = self.frame_buffer.clone();
        let stop_flag = self.stop_flag.clone();

        // Start a thread to read FFmpeg output and store frames
        thread::spawn(move || {
            let mut buffer = [0u8; 8192];

            while !stop_flag.load(Ordering::SeqCst) {
                match stdout.read(&mut buffer) {
                    Ok(0) => break, // EOF reached
                    Ok(n) => {
                        // Store the encoded data in the FrameBuffer
                        let mut fb = frame_buffer.lock().unwrap();
                        fb.push(buffer[..n].to_vec());
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(())
    }

    pub fn get_frame_buffer(&self) -> Arc<Mutex<FrameBuffer>> {
        self.frame_buffer.clone()
    }

    pub fn finish(mut self) -> Result<()> {
        self.stop_flag.store(true, Ordering::SeqCst);
        let _ = self.ffmpeg_process.kill();
        let _ = self.ffmpeg_process.wait();
        Ok(())
    }
}