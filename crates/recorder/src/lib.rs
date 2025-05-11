#![allow(dead_code)]

use log::{info, debug, warn, error};

pub mod recorder;
use recorder::Recorder;
use std::process::{Child, Command, Stdio};
use std::io::Write;

fn check_ffmpeg_installed() {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        panic!("ffmpeg not found. Please install ffmpeg and ensure it is available in your PATH.");
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32, // added buffer length in seconds
    output: String,
    temp_pattern: String, // pattern for rotating buffer files
    child: Option<Child>,
}

#[cfg(target_os = "windows")]
impl WindowsRecorder {
    pub fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, output: String) -> Self {
        // temp_pattern in temp dir
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
}

#[cfg(target_os = "windows")]
impl Recorder for WindowsRecorder {
    fn start(&mut self) {
        check_ffmpeg_installed();
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
            &self.buffer_secs.to_string(),
            "-segment_wrap",
            "1",
            "-reset_timestamps",
            "1",
            &self.temp_pattern,
        ];
        // spawn ffmpeg with piped stdin so we can send 'q' to terminate gracefully
        let mut cmd = Command::new("ffmpeg");
        cmd.args(&args)
            .stdin(Stdio::piped());
        let child = cmd.spawn().expect("Failed to start ffmpeg");
        self.child = Some(child);
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            debug!("[recorder] stop(): child present, stdin piped: {}", child.stdin.is_some());
            // attempt a graceful shutdown by sending 'q'
            if let Some(mut stdin) = child.stdin.take() {
                match stdin.write_all(b"q\n") {
                    Ok(_) => debug!("[recorder] sent 'q' to ffmpeg"),
                    Err(e) => error!("[recorder] failed to send 'q': {:?}", e),
                }
            } else {
                warn!("[recorder] no stdin to write to");
            }
            
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Forefully kill if not exited
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

    fn save(&self, path: &str) {
        // copy the rotated buffer file to final output
        let buffer_file = self.temp_pattern.replace("%03d", "000");
        match std::fs::copy(buffer_file, path) {
            Ok(_) => info!("[recorder] buffer saved to {}", path),
            Err(e) => error!("[recorder] failed to save buffer to {}: {:?}", path, e),
        }
    }
}

#[cfg(target_os = "linux")]
pub struct LinuxRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32, // added buffer length in seconds
    output: String,
    temp_pattern: String,
    child: Option<Child>,
}

#[cfg(target_os = "linux")]
impl LinuxRecorder {
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
}

#[cfg(target_os = "linux")]
impl Recorder for LinuxRecorder {
    fn start(&mut self) {
        check_ffmpeg_installed();
        let args = [
            "-y",
            "-f",
            "x11grab",
            "-framerate",
            &self.fps.to_string(),
            "-video_size",
            &format!("{}x{}", self.width, self.height),
            "-i",
            ":0.0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-f",
            "segment",
            "-segment_time",
            &self.buffer_secs.to_string(),
            "-segment_wrap",
            "1",
            "-reset_timestamps",
            "1",
            &self.temp_pattern,
        ];
        let cmd = Command::new("ffmpeg").args(&args).spawn();
        self.child = cmd.ok();
    }

    fn stop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn save(&self, path: &str) {
        let buffer_file = self.temp_pattern.replace("%03d", "000");
        let _ = std::fs::copy(buffer_file, path);
    }
}

struct OSXRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32, // added buffer length in seconds
    output: String,
    temp_pattern: String,
    child: Option<Child>,
}
impl OSXRecorder {
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
}

impl Recorder for OSXRecorder {
    fn start(&mut self) {
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
            "0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-f",
            "segment",
            "-segment_time",
            &self.buffer_secs.to_string(),
            "-segment_wrap",
            "1",
            "-reset_timestamps",
            "1",
            &self.temp_pattern,
        ];
        let cmd = Command::new("ffmpeg").args(&args).spawn();
        self.child = cmd.ok();
    }

    fn stop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn save(&self, path: &str) {
        let buffer_file = self.temp_pattern.replace("%03d", "000");
        let _ = std::fs::copy(buffer_file, path);
    }
}

/// Factory to create the appropriate recorder for the current OS
pub fn create_recorder(
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
) -> Box<dyn Recorder> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsRecorder::new(
            width,
            height,
            fps,
            buffer_secs,
            output,
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxRecorder::new(width, height, fps, buffer_secs, output))
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(OSXRecorder::new(width, height, fps, buffer_secs, output))
    }
}
