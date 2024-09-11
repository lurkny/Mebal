use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::Result;
use scrap::{Capturer, Display};
use rdev::{listen, EventType, Key};
use std::thread;

mod capture;
mod compression;
mod video_encoding;
mod upload;

use capture::Capture;

fn main() -> Result<()> {
    let display = Display::primary()?;
    let mut capturer = Capturer::new(display)?;
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    setup_hotkey_listener(stop_flag_clone);

    let mut capture = Capture::new(60, stop_flag)?;  // Specify desired FPS here
    capture.start_scrap_cap(&mut capturer)
}

fn setup_hotkey_listener(stop_flag: Arc<AtomicBool>) {
    thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            if let EventType::KeyPress(Key::F2) = event.event_type {
                println!("F2 pressed, stopping capture...");
                stop_flag.store(true, Ordering::SeqCst);
            }
        }) {
            println!("Error setting up hotkey listener: {:?}", error);
        }
    });
}