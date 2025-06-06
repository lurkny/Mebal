use async_trait::async_trait;
use log::{error, info, warn};
use std::process::Stdio;
use std::sync::Arc;
use storage::{H264Parser, ReplayBuffer};
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::check_ffmpeg_installed;
use super::recorder::Recorder;

pub struct OsxRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    child: Option<Child>,
    replay_buffer: Arc<ReplayBuffer>,
    stop_signal: Arc<Mutex<bool>>,
}

impl OsxRecorder {
    pub fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, output: String) -> Self {
        // Calculate buffer size: assume ~2 Mbps bitrate for 1080p
        let estimated_mbps = match (width, height) {
            (1920, 1080) => 2,
            (1280, 720) => 1,
            _ => 3, // Conservative estimate for higher resolutions
        };

        let max_size_mb = estimated_mbps * buffer_secs * 60 / 8; // Convert to MB
        let replay_buffer = Arc::new(ReplayBuffer::new(buffer_secs, max_size_mb as usize));

        Self {
            width,
            height,
            fps,
            buffer_secs,
            output,
            child: None,
            replay_buffer,
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl Recorder for OsxRecorder {
    fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, output: String) -> Self {
        // Calculate buffer size: assume ~2 Mbps bitrate for 1080p
        let estimated_mbps = match (width, height) {
            (1920, 1080) => 2,
            (1280, 720) => 1,
            _ => 3, // Conservative estimate for higher resolutions
        };

        let max_size_mb = estimated_mbps * buffer_secs * 60 / 8; // Convert to MB
        let replay_buffer = Arc::new(ReplayBuffer::new(buffer_secs, max_size_mb as usize));

        Self {
            width,
            height,
            fps,
            buffer_secs,
            output,
            child: None,
            replay_buffer,
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }

    async fn start(&mut self) {
        check_ffmpeg_installed();

        let args = [
            "-y",
            "-f",
            "avfoundation",
            "-framerate",
            &self.fps.to_string(),
            "-video_size",
            &format!("{}x{}", self.width, self.height),
            "-i",
            "1", // Default screen capture device (may need adjustment)
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-profile:v",
            "baseline",
            "-pix_fmt",
            "yuv420p",
            "-f",
            "h264", // Output raw H.264 stream
            "-",    // Output to stdout
        ];

        info!("[recorder] Starting macOS screen capture with FFmpeg");
        let cmd = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        match cmd {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let replay_buffer = Arc::clone(&self.replay_buffer);
                    let stop_signal = Arc::clone(&self.stop_signal);

                    tokio::task::spawn(async move {
                        let mut parser = H264Parser::new();
                        let mut reader = stdout;
                        let mut buffer = vec![0u8; 8192];

                        info!("[recorder] Started reading H.264 stream from FFmpeg (macOS)");

                        loop {
                            let should_stop = {
                                let stop = stop_signal.lock().await;
                                *stop
                            };

                            if should_stop {
                                break;
                            }

                            match reader.read(&mut buffer).await {
                                Ok(0) => {
                                    warn!("[recorder] FFmpeg stdout closed (macOS)");
                                    break;
                                }
                                Ok(n) => {
                                    let packets = parser.process_data(&buffer[..n]);
                                    for (data, is_keyframe) in packets {
                                        replay_buffer.add_packet(data, is_keyframe);
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "[recorder] Error reading from FFmpeg stdout (macOS): {:?}",
                                        e
                                    );
                                    break;
                                }
                            }
                        }

                        info!("[recorder] Stopped reading H.264 stream (macOS)");
                    });
                }
                self.child = Some(child);
                info!("[recorder] macOS screen recording started successfully");
            }
            Err(e) => {
                error!(
                    "[recorder] Failed to start FFmpeg for macOS recording: {:?}",
                    e
                );
            }
        }
    }

    async fn stop(&mut self) {
        // Signal the reading task to stop
        {
            let mut stop = self.stop_signal.lock().await;
            *stop = true;
        }

        if let Some(mut child) = self.child.take() {
            if let Err(e) = child.kill().await {
                error!("[recorder] Failed to kill ffmpeg (macOS): {:?}", e);
            } else {
                info!("[recorder] ffmpeg process killed (macOS).");
            }
            if let Err(e) = child.wait().await {
                error!("[recorder] Failed to wait for ffmpeg (macOS): {:?}", e);
            }
        } else {
            warn!("[recorder] stop(): no child to stop (macOS)");
        }

        // Reset stop signal for next recording
        {
            let mut stop = self.stop_signal.lock().await;
            *stop = false;
        }
    }

    async fn save(&self, final_output_path: &str) {
        info!(
            "[recorder] Saving replay buffer to {} (macOS)",
            final_output_path
        );

        if let Err(e) = self.replay_buffer.save_to_file(final_output_path) {
            error!("[recorder] Failed to save replay buffer (macOS): {:?}", e);
        } else {
            info!(
                "[recorder] Successfully saved replay buffer to {} (macOS)",
                final_output_path
            );
        }
    }

    fn get_output_path(&self) -> &str {
        &self.output
    }
}
