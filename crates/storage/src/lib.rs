// Author: Brody Larson

use memmap::MmapMut;
use memmap::MmapOptions;
use std::fs::OpenOptions;
use std::io::Result;
use std::io::Write;

struct Storage {
    mmap: MmapMut,
}

impl Storage {
    /// Creates a new `Storage` instance with a memory-mapped file of the specified length.
    pub fn new(len: u64) -> Result<Self> {
        assert!(len > 0, "Length must be greater than zero");

        let temp_file = std::env::temp_dir().join("mebal_storage.tmp");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(temp_file)?;

        file.set_len(len)?;

        let map = unsafe { memmap::MmapMut::map_mut(&file)? };

        Ok(Self { mmap: map })
    }
    /// Creates a new `Storage` instance with an anonymous memory-mapped region of the specified length.
    pub fn new_anon(len: u64) -> Result<Self> {
        assert!(len > 0, "Length must be greater than zero");

        let map = MmapOptions::new().len(len as usize).map_anon()?;

        Ok(Self { mmap: map })
    }
}

impl Write for Storage {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let len = buf.len();
        if len > self.mmap.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Buffer size exceeds storage size",
            ));
        }
        self.mmap[..len].copy_from_slice(buf);
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        self.mmap.flush()
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        // The MmapMut will automatically unmap when it goes out of scope
        let temp_file = std::env::temp_dir().join("mebal_storage.tmp");
        if let Err(e) = std::fs::remove_file(temp_file) {
            eprintln!("Failed to remove temporary file: {}", e);
        }
    }
}
