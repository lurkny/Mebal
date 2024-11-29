use std::io::{self, Write};
use anyhow::Result;
mod osx;
mod utils;
use osx::osx_capture::{find_display_to_capture, prepare_ffmpeg_encoder};
use std::sync::atomic::{AtomicBool, Ordering};
use rdev::{listen, EventType, Key};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use utils::storage::FrameBuffer;
use std::process::{Command, Stdio};



fn get_user_device_selection() -> Result<(String, String)> {
    // List available devices
    find_display_to_capture()?;
    
    print!("\nEnter video device ID: ");
    io::stdout().flush()?;
    let mut video_id = String::new();
    io::stdin().read_line(&mut video_id)?;
    let video_id = video_id.trim().to_string();

    print!("Enter audio device ID: ");
    io::stdout().flush()?;
    let mut audio_id = String::new();
    io::stdin().read_line(&mut audio_id)?;
    let audio_id = audio_id.trim().to_string();

    Ok((video_id, audio_id))
}

fn main() -> Result<()> {
    println!("Screen Recording Setup");
    println!("--------------------");
    
    let ids = get_user_device_selection()?;
    println!("\nSelected devices:");
    println!("Video device: {}", ids.0);
    println!("Audio device: {}", ids.1);

    // TODO: Pass these IDs to your capture implementation

    let stop_flag = Arc::new(AtomicBool::new(false));
    setup_hotkey_listener(stop_flag.clone());

    let frame_buffer = Arc::new(Mutex::new(FrameBuffer::new(1920, 1080, 10, 30)));
   let mut command =  prepare_ffmpeg_encoder(ids, "fuck.mp4")?;

   println!("Recording started. Press F2 to stop...");

    // Wait until stop_flag is set
    while !stop_flag.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }

    let mut kill = Command::new("kill")
    // TODO: replace `TERM` to signal you want.
    .args(["-s", "TERM", &command.id().to_string()])
    .spawn()?;
    kill.wait()?;

    println!("Recording stopped");

    
    Ok(())
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