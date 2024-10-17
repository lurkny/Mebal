use anyhow::{Result, Context, bail};
use std::fs::File;
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::process::{Command, Stdio, Child};
use std::thread;
use std::sync::mpsc;
use crate::compression::{CompressedFrame, decompress_frame};

pub struct FFmpegEncoder {
    ffmpeg_process: Child,
    frame_count: u64,
    fps: u32,
    error_receiver: mpsc::Receiver<String>,
}

impl FFmpegEncoder {
    pub fn new(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Self> {
        let mut ffmpeg_process = Command::new("ffmpeg")
            .args(&[
                "-y",
                "-f", "rawvideo",
                "-pixel_format", "bgra",
                "-video_size", &format!("{}x{}", width, height),
                "-framerate", &fps.to_string(),
                "-i", "raw_video.raw",  // Reading from the raw video file
                "-c:v", "libx264",
                "-preset", "medium",
                "-crf", "23",
                "-pix_fmt", "yuv420p",
                output_path,
            ])
            .stdin(Stdio::null()) 
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn FFmpeg process")?;

        let stderr = ffmpeg_process.stderr.take().expect("Failed to capture stderr");
        let (error_sender, error_receiver) = mpsc::channel();

        // Spawn a thread to capture FFmpeg's stderr output
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    // Log each line for debugging
                    eprintln!("FFmpeg stderr: {}", line);
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

    pub fn encode_frame(&mut self, frame: &CompressedFrame) -> Result<()> {
        // Decompress the frame
        let decompressed = decompress_frame(&frame.compressed_data)
            .context("Failed to decompress frame")?;

        // Write the decompressed frame data to a file (appending to raw_video.raw)
        let mut file = File::options().append(true).create(true).open("raw_video.raw")
            .context("Failed to open raw video file for writing")?;
        file.write_all(&decompressed)
            .context("Failed to write raw frame data to file")?;

        self.frame_count += 1;

        // Check for any FFmpeg errors
        if let Ok(error) = self.error_receiver.try_recv() {
            bail!("FFmpeg error: {}", error);
        }

        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        // Wait for the FFmpeg process to finish
        let status = self.ffmpeg_process.wait()
            .context("Failed to wait for FFmpeg process to exit")?;

        // Collect any remaining error messages from the error receiver
        let mut error_messages = Vec::new();
        while let Ok(error) = self.error_receiver.try_recv() {
            error_messages.push(error);
        }

        // Check if FFmpeg exited successfully
        if status.success() {
            println!("Video encoding completed successfully.");
            println!("Total frames: {}", self.frame_count);
            println!("Duration: {:.2} seconds", self.frame_count as f64 / self.fps as f64);
            Ok(())
        } else {
            let error_log = error_messages.join("\n");
            bail!("FFmpeg exited with status {}. Errors:\n{}", status, error_log);
        }
    }
}
