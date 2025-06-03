// Author: Brody Larson

use memmap::MmapMut;
use std::io::Result;

struct Storage {
    mmap: MmapMut,
}

impl Storage {
    pub fn new_anon(len: usize) -> Result<Self> {
        assert!(len > 0, "Length must be greater than zero");

        let map = memmap::MmapOptions::new().len(len).map_anon()?;
        Ok(Self { mmap: map })
    }

    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.mmap.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Write exceeds mmap length",
            ));
        }
        self.mmap[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
}
