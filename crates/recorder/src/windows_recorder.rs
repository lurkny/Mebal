use async_trait::async_trait;
use log::{debug, error, info, warn};
use std::process::Stdio;
use std::sync::Arc;
use storage::{H264Parser, ReplayBuffer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::recorder::Recorder;

pub struct WindowsRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    child: Option<Child>,
    replay_buffer: Arc<ReplayBuffer>,
    stop_signal: Arc<Mutex<bool>>,
}

#[async_trait]
impl Recorder for WindowsRecorder {
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
        super::check_ffmpeg_installed();

        // Reset stop signal
        *self.stop_signal.lock().await = false;

        let args = [
            "-y",
            "-f",
            "gdigrab",
            "-framerate",
            &self.fps.to_string(),
            "-video_size",
            &format!("{}x{}", self.width, self.height),
            "-i",
            "desktop",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-profile:v",
            "baseline", // Ensure H.264 compatibility
            "-g",
            "30", // Keyframe every 30 frames (1 second at 30fps)
            "-f",
            "h264", // Output raw H.264 stream
            "-",    // Output to stdout
        ];

        let mut cmd = Command::new("ffmpeg");
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        info!("[recorder] Starting ffmpeg with args: {:?}", args);

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let replay_buffer = Arc::clone(&self.replay_buffer);
                    let stop_signal = Arc::clone(&self.stop_signal);

                    // Spawn task to read H.264 stream from FFmpeg stdout
                    tokio::spawn(async move {
                        let mut reader = stdout;
                        let mut h264_parser = H264Parser::new();
                        let mut buffer = vec![0u8; 8192]; // 8KB buffer

                        info!("[recorder] Starting H.264 stream processing...");

                        loop {
                            // Check stop signal
                            if *stop_signal.lock().await {
                                info!("[recorder] Stop signal received, ending stream processing");
                                break;
                            }

                            // Read chunk from FFmpeg
                            match reader.read(&mut buffer).await {
                                Ok(0) => {
                                    info!("[recorder] FFmpeg stream ended");
                                    break;
                                }
                                Ok(bytes_read) => {
                                    // Parse H.264 packets and add to buffer
                                    let packets = h264_parser.process_data(&buffer[..bytes_read]);
                                    for (packet_data, is_keyframe) in packets {
                                        let packet_len = packet_data.len();
                                        replay_buffer.add_packet(packet_data, is_keyframe);
                                        if is_keyframe {
                                            debug!(
                                                "[recorder] Added keyframe packet ({} bytes)",
                                                packet_len
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("[recorder] Error reading from FFmpeg: {:?}", e);
                                    break;
                                }
                            }
                        }

                        info!("[recorder] H.264 stream processing completed");
                    });
                }

                self.child = Some(child);
                info!("[recorder] FFmpeg process started successfully");
            }
            Err(e) => {
                error!("[recorder] Failed to start ffmpeg: {:?}", e);
            }
        }
    }

    async fn stop(&mut self) {
        // Signal the stream processing task to stop
        *self.stop_signal.lock().await = true;

        if let Some(mut child) = self.child.take() {
            debug!("[recorder] Stopping FFmpeg process...");

            // Send 'q' to FFmpeg to quit gracefully
            if let Some(mut stdin) = child.stdin.take() {
                match stdin.write_all(b"q\n").await {
                    Ok(_) => debug!("[recorder] Sent 'q' to ffmpeg"),
                    Err(e) => error!("[recorder] Failed to send 'q': {:?}", e),
                }
                let _ = stdin.shutdown().await;
            }

            // Wait a bit for graceful shutdown
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Force kill if still running
            match child.try_wait() {
                Ok(Some(status)) => info!("[recorder] FFmpeg exited gracefully: {:?}", status),
                Ok(None) => {
                    warn!("[recorder] FFmpeg still running; killing now");
                    let _ = child.kill();
                    let _ = child.wait();
                }
                Err(e) => error!("[recorder] try_wait() failed: {:?}", e),
            }
        } else {
            warn!("[recorder] stop(): no child to stop");
        }
    }

    async fn save(&self, final_output_path: &str) {
        info!("[recorder] Saving replay buffer to {}", final_output_path);

        match self.replay_buffer.save_to_file(final_output_path) {
            Ok(_) => {
                info!(
                    "[recorder] Successfully saved replay buffer to {}",
                    final_output_path
                );
            }
            Err(e) => {
                error!("[recorder] Failed to save replay buffer: {:?}", e);
            }
        }
    }

    fn get_output_path(&self) -> &str {
        &self.output
    }
}
