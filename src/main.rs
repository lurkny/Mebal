#![warn(clippy::pedantic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::Result;
use rdev::{listen, EventType, Key};
use std::thread;

mod capture_strategy;
mod compression;
mod video_encoding_strategy;
mod upload;

#[cfg(target_os = "windows")]
mod windows_capture;
#[cfg(target_os = "windows")]
mod windows_encoder;

#[cfg(not(target_os = "windows"))]
mod generic_capture;
#[cfg(not(target_os = "windows"))]
mod ffmpeg_encoder;

use capture_strategy::CaptureStrategy;

fn main() -> Result<()> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    setup_hotkey_listener(stop_flag_clone);

    let mut capture_strategy = CaptureStrategy::new(60, stop_flag)?;  // Specify  FPS here
    capture_strategy.start_capture()
}

fn setup_hotkey_listener(stop_flag: Arc<AtomicBool>) {
    thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            if let EventType::KeyPress(Key::F2) = event.event_type {
                println!("F2 pressed, stopping capture...");
                stop_flag.store(true, Ordering::SeqCst);
            }
        }) {
            println!("Error setting up hotkey listener: {error:?}");
        }
    });
}