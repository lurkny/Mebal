use anyhow::Result;
use windows_capture::encoder::{
    AudioSettingBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
};
use windows::Foundation::TimeSpan;

use crate::compression::{CompressedFrame, decompress_frame, convert_to_bottom_up};

pub struct WindowsEncoder {
    encoder: VideoEncoder,
}

impl WindowsEncoder {
    pub fn new(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Self> {
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height)
                .frame_rate(fps)
                .bitrate(10_000_000),  // 10 Mbps, adjust as needed
            AudioSettingBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            output_path
        )?;

        Ok(Self { encoder })
    }

    pub fn encode_frame(&mut self, frame: &CompressedFrame) -> Result<()> {
        let mut decompressed = decompress_frame(&frame.compressed_data)?;
        convert_to_bottom_up(&mut decompressed, frame.width, frame.height);
        let frame_time = TimeSpan::from(frame.timestamp);
        self.encoder.send_frame_buffer(&decompressed, frame_time.Duration)?;
        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        self.encoder.finish()?;
        Ok(())
    }
}