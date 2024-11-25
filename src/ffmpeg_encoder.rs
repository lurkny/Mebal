use anyhow::{Result, Context, bail};
use std::process::{Command, Stdio, Child};
use std::thread;
use std::sync::mpsc;
use std::io::{Write};
use crate::compression::decompress_frame;

pub struct FFmpegEncoder {
    ffmpeg_process: Child,
    frame_count: u64,
    fps: u32,
    error_receiver: mpsc::Receiver<String>,
}

impl FFmpegEncoder {
    pub fn new(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Self> {
        // Round height up to nearest even number
        let adjusted_height = (height + 1) & !1;
        
        let mut ffmpeg_process = Command::new("ffmpeg")
            .args(&[
                "-y",                                    // Overwrite output file if it exists
                "-f", "rawvideo",                       // Input format is raw video
                "-pixel_format", "bgra",                // Input pixel format is BGRA
                "-video_size", &format!("{}x{}", width, adjusted_height),
                "-framerate", &fps.to_string(),
                "-i", "-",                             // Read from stdin instead of file
                "-c:v", "libx264",                     // Use H.264 codec
                "-preset", "ultrafast",                // Fastest encoding preset
                "-tune", "zerolatency",               // Optimize for low-latency
                "-crf", "23",                         // Constant rate factor (quality)
                "-pix_fmt", "yuv420p",                // Output pixel format
                "-movflags", "+faststart",            // Enable fast start for web playback
                output_path
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn FFmpeg process")?;

        let stderr = ffmpeg_process.stderr.take().expect("Failed to capture stderr");
        let (error_sender, error_receiver) = mpsc::channel();

        // Spawn a thread to capture FFmpeg's stderr output
        thread::spawn(move || {
            use std::io::{BufReader, BufRead};
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("FFmpeg: {}", line);
                    error_sender.send(line).expect("Failed to send error");
                }
            }
        });

        Ok(Self {
            ffmpeg_process,
            frame_count: 0,
            fps,
            error_receiver,
        })
    }

    pub fn encode_frame(&mut self, frame_data: &[u8]) -> Result<()> {
        // Get stdin handle for writing frame data
        let decompressed_data = decompress_frame(frame_data)?;

        let stdin = self.ffmpeg_process.stdin.as_mut()
            .context("Failed to get stdin handle")?;

        // Write the frame directly to FFmpeg's stdin
        stdin.write_all(&decompressed_data)
            .context("Failed to write frame data to FFmpeg")?;

        self.frame_count += 1;

        // Check for any FFmpeg errors
        if let Ok(error) = self.error_receiver.try_recv() {
            if error.contains("Error") || error.contains("error") {
                bail!("FFmpeg error: {}", error);
            }
        }

        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        // Close stdin to signal end of input
        drop(self.ffmpeg_process.stdin.take());

        // Wait for the FFmpeg process to finish
        let status = self.ffmpeg_process.wait()
            .context("Failed to wait for FFmpeg process to exit")?;

        // Collect any remaining error messages
        let mut error_messages = Vec::new();
        while let Ok(error) = self.error_receiver.try_recv() {
            error_messages.push(error);
        }

        if status.success() {
            println!("Video encoding completed successfully");
            println!("Total frames: {}", self.frame_count);
            println!("Duration: {:.2} seconds", self.frame_count as f64 / self.fps as f64);
            Ok(())
        } else {
            let error_log = error_messages.join("\n");
            bail!("FFmpeg exited with status {}. Errors:\n{}", status, error_log)
        }
    }
}