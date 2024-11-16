use std::sync::{Arc, atomic::AtomicBool};
use anyhow::Result;

#[cfg(target_os = "windows")]
use crate::windows_capture::WindowsCapture;

#[cfg(not(target_os = "windows"))]
use crate::generic_capture::GenericCapture;

pub enum CaptureStrategy {
    #[cfg(target_os = "windows")]
    Windows(WindowsCapture),
    #[cfg(not(target_os = "windows"))]
    Generic(GenericCapture),
}

impl CaptureStrategy {
    pub fn new(fps: u32, stop_flag: Arc<AtomicBool>) -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            Ok(CaptureStrategy::Windows(WindowsCapture::new(fps, stop_flag)?))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(CaptureStrategy::Generic(GenericCapture::new(fps, stop_flag)?))
        }
    }

    pub fn start_capture(&mut self) -> Result<()> {
        match self {
            #[cfg(target_os = "windows")]
            CaptureStrategy::Windows(capture) => capture.start_duplication_cap(),
            #[cfg(not(target_os = "windows"))]
            CaptureStrategy::Generic(capture) => capture.start_capture(),
        }
    }
}