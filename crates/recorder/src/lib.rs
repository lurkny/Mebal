#![allow(dead_code)]

use std::{path::PathBuf, process::Command};

pub mod linux_recorder;
pub mod osx_recorder;
pub mod recorder;
pub mod windows_recorder;

use anyhow::Result;
use recorder::Recorder;

fn check_ffmpeg_installed() {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        panic!("ffmpeg not found. Please install ffmpeg and ensure it is available in your PATH.");
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

pub fn collect_segments() -> Result<PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(std::env::temp_dir())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("replay_buffer_")
        })
        .collect();

    entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

    let list_path = std::env::temp_dir().join("list.txt");
    let mut list_file = std::fs::File::create(&list_path)?;

    for entry in entries {
        let line = format!("file '{}'\n", entry.path().display());
        use std::io::Write;
        list_file.write_all(line.as_bytes())?;
    }

    Ok(list_path)
}

fn assemble_segments(list_path: &PathBuf, final_output_path: &str) -> Result<()> {
    let output_path = PathBuf::from(final_output_path); // Use the final_output_path argument
    let output_dir = output_path.parent().unwrap();
    std::fs::create_dir_all(output_dir)?;

    let args = [
        "-y",
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        list_path.to_str().unwrap(),
        "-c",
        "copy",
        &output_path.to_str().unwrap(),
    ];

    Command::new("ffmpeg")
        .args(&args)
        .output()?;

    Ok(())
}
