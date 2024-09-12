use std::collections::VecDeque;
use anyhow::Result;
use windows_capture::encoder::{
    AudioSettingBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
};
use windows::Foundation::TimeSpan;

use crate::compression::{CompressedFrame, decompress_frame, convert_to_bottom_up};

pub fn save_buffer(frame_buffer: &VecDeque<CompressedFrame>, fps: u32) -> Result<()> {
    let mut encoder = VideoEncoder::new(
        VideoSettingsBuilder::new(frame_buffer[0].width, frame_buffer[0].height)
            .frame_rate(fps)
            .bitrate(10_000_000),  // 5 Mbps, adjust as needed
        AudioSettingBuilder::default().disabled(true),
        ContainerSettingsBuilder::default(),
        "output.mp4"
    )?;

    for frame in frame_buffer.iter() {
        let mut decompressed = decompress_frame(&frame.compressed_data)?;
        convert_to_bottom_up(&mut decompressed, frame.width, frame.height);
        let frame_time = TimeSpan::from(frame.timestamp);
        encoder.send_frame_buffer(&decompressed, frame_time.Duration)?;
    }

    encoder.finish()?;
    Ok(())
}