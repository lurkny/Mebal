use log::{debug, error, info, warn};
use std::io::Write;
use std::process::{Child, Command, Stdio};

use super::check_ffmpeg_installed;
use super::recorder::Recorder;

pub struct WindowsRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    temp_pattern: String,
    child: Option<Child>,
}

impl WindowsRecorder {
    pub fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, output: String) -> Self {
        let temp_pattern = std::env::temp_dir()
            .join("replay_buffer_%03d.mp4")
            .to_string_lossy()
            .into_owned();
        Self {
            width,
            height,
            fps,
            buffer_secs,
            output,
            temp_pattern,
            child: None,
        }
    }

    fn cleanup_temp_files(&self) {
        info!("[recorder] Cleaning up temporary segment files...");
        let temp_dir = std::env::temp_dir();
        match std::fs::read_dir(temp_dir) {
            Ok(entries) => {
                for entry in entries.filter_map(|e| e.ok()) {
                    let file_name = entry.file_name().to_string_lossy().into_owned();
                    if file_name.starts_with("replay_buffer_") && file_name.ends_with(".mp4") {
                        if let Err(e) = std::fs::remove_file(entry.path()) {
                            warn!("[recorder] Failed to remove temp file {:?}: {:?}", entry.path(), e);
                        } else {
                            debug!("[recorder] Removed temp file {:?}", entry.path());
                        }
                    }
                }
            }
            Err(e) => {
                error!("[recorder] Failed to read temp directory for cleanup: {:?}", e);
            }
        }
    }
}

impl Recorder for WindowsRecorder {
    fn start(&mut self) {
        check_ffmpeg_installed();
        // Ensure previous temp files are cleaned up before starting a new recording session
        self.cleanup_temp_files();

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
            "-f",
            "segment",
            "-segment_time",
            "1", // Record 1-second segments
            "-segment_wrap",
            &self.buffer_secs.to_string(), // Keep <buffer_secs> worth of segments
            "-reset_timestamps",
            "1", // Reset timestamps for each segment
            &self.temp_pattern, // Output to replay_buffer_%03d.mp4
        ];
        let mut cmd = Command::new("ffmpeg");
        cmd.args(&args).stdin(Stdio::piped());
        info!("[recorder] Starting ffmpeg with args: {:?}", args);
        let child = cmd.spawn().expect("Failed to start ffmpeg");
        self.child = Some(child);
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            debug!(
                "[recorder] stop(): child present, stdin piped: {}",
                child.stdin.is_some()
            );
            if let Some(mut stdin) = child.stdin.take() {
                match stdin.write_all(
                    b"q
",
                ) {
                    Ok(_) => debug!("[recorder] sent 'q' to ffmpeg"),
                    Err(e) => error!("[recorder] failed to send 'q': {:?}", e),
                }
            } else {
                warn!("[recorder] no stdin to write to");
            }

            std::thread::sleep(std::time::Duration::from_millis(500));

            match child.try_wait() {
                Ok(Some(status)) => info!("[recorder] ffmpeg exited gracefully: {:?}", status),
                Ok(None) => {
                    warn!("[recorder] ffmpeg still running; killing now");
                    let _ = child.kill();
                    let _ = child.wait();
                }
                Err(e) => error!("[recorder] try_wait() failed: {:?}", e),
            }
        } else {
            warn!("[recorder] stop(): no child to stop");
        }
    }

    fn save(&self, final_output_path: &str) {
        info!("[recorder] Attempting to save buffer to {}", final_output_path);
        match super::collect_segments() {
            Ok(list_path) => {
                info!("[recorder] Collected segments into: {:?}", list_path);
                match super::assemble_segments(&list_path, final_output_path) {
                    Ok(_) => {
                        info!("[recorder] Assembled segments successfully to {}", final_output_path);
                    }
                    Err(e) => {
                        error!("[recorder] Failed to assemble segments: {:?}", e);
                    }
                }
                // Clean up the list file
                if let Err(e) = std::fs::remove_file(&list_path) {
                    warn!("[recorder] Failed to remove list file {:?}: {:?}", list_path, e);
                }
            }
            Err(e) => {
                error!("[recorder] Failed to collect segments: {:?}", e);
            }
        }
        // Clean up individual segment files after assembly (or if assembly failed but list was created)
        self.cleanup_temp_files();
    }

    fn get_output_path(&self) -> &str {
        &self.output
    }
}
