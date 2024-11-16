use anyhow::Result;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use std::time::Duration;

pub struct CompressedFrame {
    pub compressed_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: Duration,
}

pub fn compress_frame(buffer: &[u8]) -> Result<Vec<u8>> {
    // lz4_flex's compress_prepend_size handles the size prefix automatically
    Ok(compress_prepend_size(buffer))
}

pub fn decompress_frame(compressed_data: &[u8]) -> Result<Vec<u8>> {
    // decompress_size_prepended automatically reads the prepended size
    decompress_size_prepended(compressed_data)
        .map_err(|e| anyhow::anyhow!("Decompression error: {}", e))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_roundtrip() {
        let original = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let compressed = compress_frame(&original).unwrap();
        let decompressed = decompress_frame(&compressed).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_bottom_up_conversion() {
        // Test with a 2x2 RGBA image (32bpp)
        let mut buffer = vec![
            1, 2, 3, 4,    5, 6, 7, 8,     // Top row
            9, 10, 11, 12, 13, 14, 15, 16  // Bottom row
        ];
        let width = 2;
        let height = 2;
        
        convert_to_bottom_up(&mut buffer, width, height);
        
        let expected = vec![
            9, 10, 11, 12, 13, 14, 15, 16,  // Original bottom row now at top
            1, 2, 3, 4,    5, 6, 7, 8      // Original top row now at bottom
        ];
        
        assert_eq!(buffer, expected);
    }
}