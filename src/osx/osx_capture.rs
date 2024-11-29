use std::process::{Command, Stdio, Child};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use anyhow::{Result, Context};

/// Prepares the FFmpeg encoder for capturing frames by spawning a new FFmpeg process.
/// Captured video and audio are saved directly to the specified output file.
pub fn prepare_ffmpeg_encoder(
    device_ids: (String, String),
    output_file: &str,
) -> Result<Child> {
    println!("Preparing FFmpeg encoder...");

    let (video_id, audio_id) = device_ids;

    // Construct the FFmpeg command with supported pixel format and video codec
    let ffmpeg_command = Command::new("ffmpeg")
        .args(&[
            "-f", "avfoundation",
            "-y",
            "-i", &format!("{}:{}", video_id, audio_id),
            // Video encoding settings
            "-c:v", "h264_videotoolbox", // Hardware-accelerated H.264 encoding
            "-pix_fmt", "nv12",        // Supported pixel format
            // Audio encoding settings
            "-c:a", "aac",                // AAC audio codec
            "-b:a", "128k",               // Audio bitrate
            "-f", "mp4",
            "-movflags", "+faststart",    // Fast start for streaming
            // Output file
            output_file,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null()) // No need to pipe stdout
        .stderr(Stdio::inherit()) // Inherit stderr for debugging
        .spawn()
        .context("Failed to spawn FFmpeg process")?;

    println!("FFmpeg encoder started. Output file: {}", output_file);

    Ok(ffmpeg_command)
}


/// Finds and lists available video and audio devices for capture.
/// Outputs only video and audio devices.

pub fn find_display_to_capture() -> Result<(), std::io::Error> {
    println!("Finding available capture devices...\n");

    let output = Command::new("ffmpeg")
        .args(&["-f", "avfoundation", "-list_devices", "true", "-i", ""])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Split into lines and process
    let mut video_devices = Vec::new();
    let mut audio_devices = Vec::new();
    let mut is_video = false;
    let mut is_audio = false;

    for line in stderr.lines() {
        if line.contains("AVFoundation video devices:") {
            is_video = true;
            is_audio = false;
            println!("\nVideo Devices:");
            continue;
        } else if line.contains("AVFoundation audio devices:") {
            is_video = false;
            is_audio = true;
            println!("\nAudio Devices:");
            continue;
        }

        // Parse device lines
        if let Some(device) = line.strip_prefix("[AVFoundation indev @ ") {
            if let Some(idx_start) = device.find("] [") {
                if let Some(idx_end) = device.find("] ") {
                    let device_name = &device[idx_end + 2..];
                    let device_id = &device[idx_start + 3..idx_start + 4];
                    
                    if is_video {
                        video_devices.push((device_id, device_name));
                        println!("  {}: {}", device_id, device_name);
                    } else if is_audio {
                        audio_devices.push((device_id, device_name));
                        println!("  {}: {}", device_id, device_name);
                    }
                }
            }
        }
    }

    Ok(())
}