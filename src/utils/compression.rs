use anyhow::{Result, Context};
use std::process::{Command, Stdio};
use std::io::{Write};
use std::time::Duration;

pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: Duration,
}

impl EncodedFrame {
    pub fn new(data: Vec<u8>, width: u32, height: u32, timestamp: Duration) -> Self {
        Self { data, width, height, timestamp }
    }
}

/// Converts a BGRA frame to YUV420p format using FFmpeg
/// 
/// # Arguments
///  * `bgra_frame` - The BGRA frame data
/// * `width` - The width of the frame
/// * `height` - The height of the frame
/// 
/// # Returns
/// A `YUVFrame` struct containing the YUV420p frame data

/* 
pub fn convert_frame_toyuv420(bgra_frame: &[u8], width: u32, height: u32) -> Result<EncodedFrame> {
    let mut ffmpeg = Command::new("ffmpeg")
    .args(&[
        "-f", "rawvideo",
        "-pixel_format", "bgra",
        "-video_size", &format!("{}x{}", width, height),
        "-i", "pipe:0",
        "-f", "rawvideo",
        "-pix_fmt", "yuv420p",
        "pipe:1",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .context("Failed to spawn FFmpeg process")?;
    
    let stdin = ffmpeg.stdin.as_mut().expect("Failed to open stdin");
    stdin.write_all(bgra_frame).context("Failed to write to FFmpeg stdin")?;
    let output = ffmpeg.wait_with_output().context("Failed to read FFmpeg output")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("FFmpeg failed with exit code: {}", output.status));
    }

    let yuv_frame = EncodedFrame::new(output.stdout, width, height);

    Ok(yuv_frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_convert_frame_toyuv240() -> Result<()> {
        // Create a sample 2x2 BGRA frame (4 pixels)
        let width = 2;
        let height = 2;
        let bgra_frame = vec![
            0, 0, 255, 255, // Blue pixel
            0, 255, 0, 255, // Green pixel
            255, 0, 0, 255, // Red pixel
            255, 255, 255, 255, // White pixel
        ];

        // Call the function
        let yuv_frame = convert_frame_toyuv420(&bgra_frame, width, height)?;

        // Verify the output
        // YUV420p format for a 2x2 image should have 6 bytes:
        // 4 bytes for Y plane (one byte per pixel)
        // 1 byte for U plane (one byte for 2x2 block)
        // 1 byte for V plane (one byte for 2x2 block)
        assert_eq!(yuv_frame.data.len(), 6);

        Ok(())
    }
}

*/