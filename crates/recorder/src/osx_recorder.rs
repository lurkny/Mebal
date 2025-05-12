use std::process::{Child, Command, Stdio};

use log::{debug, error, info, warn};

use super::check_ffmpeg_installed;
use super::recorder::Recorder;

pub struct OsxRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    temp_pattern: String,
    child: Option<Child>,
}

impl OsxRecorder {
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
        info!("[recorder] Cleaning up temporary segment files (macOS)...");
        let temp_dir = std::env::temp_dir();
        match std::fs::read_dir(temp_dir) {
            Ok(entries) => {
                for entry in entries.filter_map(|e| e.ok()) {
                    let file_name = entry.file_name().to_string_lossy().into_owned();
                    if file_name.starts_with("replay_buffer_") && file_name.ends_with(".mp4") {
                        if let Err(e) = std::fs::remove_file(entry.path()) {
                            warn!("[recorder] Failed to remove temp file {:?} (macOS): {:?}", entry.path(), e);
                        } else {
                            debug!("[recorder] Removed temp file {:?} (macOS)", entry.path());
                        }
                    }
                }
            }
            Err(e) => {
                error!("[recorder] Failed to read temp directory for cleanup (macOS): {:?}", e);
            }
        }
    }
}

impl Recorder for OsxRecorder {
    fn start(&mut self) {
        check_ffmpeg_installed();
        self.cleanup_temp_files(); // Clean up before starting

        let args = [
            "-y",
            "-f",
            "avfoundation",
            "-framerate",
            &self.fps.to_string(),
            "-video_size",
            &format!("{}x{}", self.width, self.height),
            "-i",
            "0", // Consider making display configurable
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
        info!("[recorder] Starting ffmpeg with args: {:?}", args);
        let cmd = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        self.child = cmd.ok();
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if let Err(e) = child.kill() {
                error!("[recorder] Failed to kill ffmpeg (macOS): {:?}", e);
            } else {
                info!("[recorder] ffmpeg process killed (macOS).");
            }
            if let Err(e) = child.wait() {
                error!("[recorder] Failed to wait for ffmpeg (macOS): {:?}", e);
            }
        } else {
            warn!("[recorder] stop(): no child to stop (macOS)");
        }
    }

    fn save(&self, final_output_path: &str) {
        info!("[recorder] Attempting to save buffer to {} (macOS)", final_output_path);
        match super::collect_segments() {
            Ok(list_path) => {
                info!("[recorder] Collected segments into: {:?} (macOS)", list_path);
                match super::assemble_segments(&list_path, final_output_path) {
                    Ok(_) => {
                        info!("[recorder] Assembled segments successfully to {} (macOS)", final_output_path);
                    }
                    Err(e) => {
                        error!("[recorder] Failed to assemble segments (macOS): {:?}", e);
                    }
                }
                if let Err(e) = std::fs::remove_file(&list_path) {
                    warn!("[recorder] Failed to remove list file {:?} (macOS): {:?}", list_path, e);
                }
            }
            Err(e) => {
                error!("[recorder] Failed to collect segments (macOS): {:?}", e);
            }
        }
        self.cleanup_temp_files();
    }

    fn get_output_path(&self) -> &str {
        &self.output
    }
}
