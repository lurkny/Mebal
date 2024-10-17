use anyhow::Result;
use lz4::{Decoder, EncoderBuilder};
use std::time::Duration;
use std::io::{Read, Write};

pub struct CompressedFrame {
    pub compressed_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: Duration,
}


pub fn compress_frame(buffer: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = EncoderBuilder::new().level(0).favor_dec_speed(true).build(Vec::new())?;
    encoder.write_all(buffer)?;
    let (compressed_data, result) = encoder.finish();
    result.map_err(|e| e.into()).map(|_| compressed_data)
}

pub fn decompress_frame(compressed_data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = Decoder::new(compressed_data)?;
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

pub fn convert_to_bottom_up(buffer: &mut [u8], width: u32, height: u32) {
    let stride = width as usize * 4;
    let half_height = height as usize / 2;
    for y in 0..half_height {
        let top = y * stride;
        let bottom = (height as usize - 1 - y) * stride;
        let (top_slice, bottom_slice) = buffer.split_at_mut(bottom);
        top_slice[top..top + stride].swap_with_slice(&mut bottom_slice[..stride]);
    }
}