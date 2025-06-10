#![allow(dead_code)]

use std::process::Command;

pub mod avdict;
pub mod codecpar;
pub mod linux_recorder;
pub mod osx_recorder;
pub mod recorder;
pub mod utils;
pub mod windows_recorder;

use recorder::Recorder;

pub fn init() {}

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
        Box::new(windows_recorder::WindowsRecorder::new(
            width,
            height,
            fps,
            buffer_secs,
            output,
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux_recorder::LinuxRecorder::new(
            width,
            height,
            fps,
            buffer_secs,
            output,
        ))
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(osx_recorder::OsxRecorder::new(
            width,
            height,
            fps,
            buffer_secs,
            output,
        ))
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        panic!("Unsupported OS for recording");
    }
}
