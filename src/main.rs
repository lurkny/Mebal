pub use anyhow;
use dioxus::{
    desktop::{Config, WindowBuilder},
    prelude::*,
};
pub use env_logger;
use log::{debug, error, info, warn};
use rdev::{listen, EventType, Key};
use recorder::create_recorder;
use std::path::PathBuf;

static CSS: Asset = asset!("/assets/main.css");

#[derive(PartialEq, Debug, Clone)]
struct RecordingConfig {
    resolution: Signal<String>,
    fps: Signal<String>,
    output_path: Signal<String>,
    buffer_secs: Signal<String>,
    hotkey: Signal<String>,
    listener_started: Signal<bool>,
}

impl RecordingConfig {
    fn new() -> Self {
        // Get user's video directory or fallback to home/Downloads
        let video_dir = get_user_video_directory();
        let default_output = video_dir
            .join("mebal_recording.mp4")
            .to_string_lossy()
            .to_string();

        Self {
            resolution: Signal::new("1920x1080".to_string()),
            fps: Signal::new("60".to_string()),
            output_path: Signal::new(default_output),
            buffer_secs: Signal::new("30".to_string()),
            hotkey: Signal::new("F3".to_string()),
            listener_started: Signal::new(false),
        }
    }
}

fn get_user_video_directory() -> PathBuf {
    // Try to get the user's video directory
    if let Some(home_dir) = std::env::var_os("HOME") {
        let video_path = PathBuf::from(home_dir).join("Movies");
        if video_path.exists() {
            return video_path;
        }
    }

    // Fallback to Downloads directory
    if let Some(home_dir) = std::env::var_os("HOME") {
        let downloads_path = PathBuf::from(home_dir).join("Downloads");
        if downloads_path.exists() {
            return downloads_path;
        }
    }

    // Final fallback to current directory
    PathBuf::from(".")
}

fn string_to_key(key_str: &str) -> Option<Key> {
    match key_str.to_uppercase().as_str() {
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "SPACE" => Some(Key::Space),
        "ENTER" => Some(Key::Return),
        "TAB" => Some(Key::Tab),
        "ESC" | "ESCAPE" => Some(Key::Escape),
        "BACKSPACE" => Some(Key::Backspace),
        "DELETE" => Some(Key::Delete),
        "INSERT" => Some(Key::Insert),
        "HOME" => Some(Key::Home),
        "END" => Some(Key::End),
        "PAGEUP" => Some(Key::PageUp),
        "PAGEDOWN" => Some(Key::PageDown),
        "UP" => Some(Key::UpArrow),
        "DOWN" => Some(Key::DownArrow),
        "LEFT" => Some(Key::LeftArrow),
        "RIGHT" => Some(Key::RightArrow),
        _ => {
            // Try single character keys
            if key_str.len() == 1 {
                let ch = key_str.chars().next().unwrap().to_ascii_uppercase();
                match ch {
                    'A' => Some(Key::KeyA),
                    'B' => Some(Key::KeyB),
                    'C' => Some(Key::KeyC),
                    'D' => Some(Key::KeyD),
                    'E' => Some(Key::KeyE),
                    'F' => Some(Key::KeyF),
                    'G' => Some(Key::KeyG),
                    'H' => Some(Key::KeyH),
                    'I' => Some(Key::KeyI),
                    'J' => Some(Key::KeyJ),
                    'K' => Some(Key::KeyK),
                    'L' => Some(Key::KeyL),
                    'M' => Some(Key::KeyM),
                    'N' => Some(Key::KeyN),
                    'O' => Some(Key::KeyO),
                    'P' => Some(Key::KeyP),
                    'Q' => Some(Key::KeyQ),
                    'R' => Some(Key::KeyR),
                    'S' => Some(Key::KeyS),
                    'T' => Some(Key::KeyT),
                    'U' => Some(Key::KeyU),
                    'V' => Some(Key::KeyV),
                    'W' => Some(Key::KeyW),
                    'X' => Some(Key::KeyX),
                    'Y' => Some(Key::KeyY),
                    'Z' => Some(Key::KeyZ),
                    '0' => Some(Key::Num0),
                    '1' => Some(Key::Num1),
                    '2' => Some(Key::Num2),
                    '3' => Some(Key::Num3),
                    '4' => Some(Key::Num4),
                    '5' => Some(Key::Num5),
                    '6' => Some(Key::Num6),
                    '7' => Some(Key::Num7),
                    '8' => Some(Key::Num8),
                    '9' => Some(Key::Num9),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

fn main() {
    env_logger::init();

    let d_cfg = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("Mebal - Configurable Replay Buffer")
                .with_decorations(true)
                .with_inner_size(dioxus::desktop::LogicalSize::new(500.0, 400.0)),
        )
        .with_disable_context_menu(true);

    dioxus::LaunchBuilder::desktop().with_cfg(d_cfg).launch(app);
}

pub fn app() -> Element {
    let _config = use_context_provider(|| RecordingConfig::new());

    rsx! {
        document::Stylesheet { href: CSS }
        div { class: "app-container",
            h1 { class: "app-title", "Mebal Configuration" }
            div { class: "config-form",
                ResolutionInput {}
                FpsInput {}
                BufferSecondsInput {}
                HotkeyInput {}
                OutputPathInput {}
                StartBufferButton {}
                StatusDisplay {}
            }
        }
    }
}

#[component]
fn ResolutionInput() -> Element {
    let mut res = use_context::<RecordingConfig>().resolution;
    rsx! {
        div { class: "form-group",
            label { "Resolution:" }
            select {
                value: "{res}",
                onchange: move |e| res.set(e.value()),
                option { value: "1280x720", "720p (1280x720)" }
                option { value: "1920x1080", "1080p (1920x1080)" }
                option { value: "2560x1440", "1440p (2560x1440)" }
                option { value: "3840x2160", "4K (3840x2160)" }
            }
            small { class: "form-help", "Select your desired recording resolution" }
        }
    }
}

#[component]
fn FpsInput() -> Element {
    let mut fps = use_context::<RecordingConfig>().fps;
    rsx! {
        div { class: "form-group",
            label { "Frame Rate (FPS):" }
            select {
                value: "{fps}",
                onchange: move |e| fps.set(e.value()),
                option { value: "24", "24 FPS (Cinema)" }
                option { value: "30", "30 FPS (Standard)" }
                option { value: "60", "60 FPS (Smooth)" }
                option { value: "120", "120 FPS (High)" }
            }
            small { class: "form-help", "Higher FPS = smoother video but larger files" }
        }
    }
}

#[component]
fn BufferSecondsInput() -> Element {
    let mut buffer_secs = use_context::<RecordingConfig>().buffer_secs;
    rsx! {
        div { class: "form-group",
            label { "Buffer Duration (seconds):" }
            input {
                r#type: "number",
                value: "{buffer_secs}",
                oninput: move |e| buffer_secs.set(e.value()),
                min: "5",
                max: "300",
                step: "5"
            }
            small { class: "form-help", "How many seconds to keep in memory (5-300 seconds)" }
        }
    }
}

#[component]
fn HotkeyInput() -> Element {
    let mut hotkey = use_context::<RecordingConfig>().hotkey;
    rsx! {
        div { class: "form-group",
            label { "Save Hotkey:" }
            select {
                value: "{hotkey}",
                onchange: move |e| hotkey.set(e.value()),
                option { value: "F3", "F3 (Recommended)" }
                option { value: "F4", "F4" }
                option { value: "F5", "F5" }
                option { value: "F6", "F6" }
                option { value: "F7", "F7" }
                option { value: "F8", "F8" }
                option { value: "F9", "F9" }
                option { value: "F10", "F10" }
                option { value: "F11", "F11" }
                option { value: "F12", "F12" }
                option { value: "SPACE", "Spacebar" }
                option { value: "P", "P Key" }
                option { value: "S", "S Key" }
            }
            small { class: "form-help", "Press this key to save the current buffer" }
        }
    }
}

#[component]
fn OutputPathInput() -> Element {
    let mut output_path = use_context::<RecordingConfig>().output_path;
    rsx! {
        div { class: "form-group",
            label { "Output File Path:" }
            input {
                r#type: "text",
                value: "{output_path}",
                oninput: move |e| output_path.set(e.value()),
                placeholder: "Path where recordings will be saved"
            }
            button {
                r#type: "button",
                onclick: move |_| {
                    let video_dir = get_user_video_directory();
                    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                    let new_path = video_dir.join(format!("mebal_{}.mp4", timestamp)).to_string_lossy().to_string();
                    output_path.set(new_path);
                },
                "Generate New Filename"
            }
            small { class: "form-help", "Files will be saved to your Videos folder by default" }
        }
    }
}

#[component]
fn StartBufferButton() -> Element {
    let mut listener_started = use_context::<RecordingConfig>().listener_started;
    let resolution_sig = use_context::<RecordingConfig>().resolution;
    let fps_sig = use_context::<RecordingConfig>().fps;
    let output_path_sig = use_context::<RecordingConfig>().output_path;
    let buffer_secs_sig = use_context::<RecordingConfig>().buffer_secs;
    let hotkey_sig = use_context::<RecordingConfig>().hotkey;

    rsx! {
        div { class: "form-group",
            button {
                class: if *listener_started.read() { "button-stop" } else { "button-start" },
                onclick: move |_| {
                    if !*listener_started.read() {
                        let resolution = resolution_sig.read().clone();
                        let fps = fps_sig.read().clone();
                        let output_path = output_path_sig.read().clone();
                        let buffer_secs = buffer_secs_sig.read().clone();
                        let hotkey = hotkey_sig.read().clone();

                        if let Err(e) = start_recording(&resolution, &fps, &output_path, &buffer_secs, &hotkey) {
                            error!("Failed to start recording: {}", e);
                        } else {
                            listener_started.set(true);
                        }
                    } else {
                        // TODO: Implement stop functionality
                        warn!("Stop functionality not yet implemented");
                    }
                },
                if *listener_started.read() { "Stop Buffer" } else { "Start Buffer" }
            }
        }
    }
}

#[component]
fn StatusDisplay() -> Element {
    let listener_started = use_context::<RecordingConfig>().listener_started;
    let hotkey = use_context::<RecordingConfig>().hotkey;

    rsx! {
        div { class: "status-display",
            if *listener_started.read() {
                div { class: "status-active",
                    "🔴 Recording active - Press {hotkey} to save buffer"
                }
            } else {
                div { class: "status-inactive",
                    "⚫ Click 'Start Buffer' to begin recording"
                }
            }
        }
    }
}

fn start_recording(
    resolution: &str,
    fps: &str,
    output_path: &str,
    buffer_secs: &str,
    hotkey_str: &str,
) -> anyhow::Result<()> {
    // Validate hotkey
    let target_key = string_to_key(hotkey_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid hotkey: {}", hotkey_str))?;

    // Clone all inputs into owned strings for the spawned task
    let resolution = resolution.to_string();
    let fps = fps.to_string();
    let buffer_secs = buffer_secs.to_string();
    let output_path_for_thread = output_path.to_string();
    let hotkey_display = hotkey_str.to_string();

    std::thread::spawn(move || {
        // Create a new Tokio runtime for this thread
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("[recorder] Failed to create Tokio runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            // Parse parameters with error handling
            let (width, height) = match parse_resolution(&resolution) {
                Ok((w, h)) => (w, h),
                Err(e) => {
                    error!("[recorder] Invalid resolution '{}': {}", resolution, e);
                    return;
                }
            };

            let fps_val = match fps.parse::<u32>() {
                Ok(f) => f,
                Err(_) => {
                    error!("[recorder] Invalid FPS '{}': must be a number", fps);
                    return;
                }
            };

            let buffer_secs_val = match buffer_secs.parse::<u32>() {
                Ok(b) => b,
                Err(_) => {
                    error!(
                        "[recorder] Invalid buffer seconds '{}': must be a number",
                        buffer_secs
                    );
                    return;
                }
            };

            // Create & start ffmpeg recorder
            info!(
                "[recorder] Starting: {}x{} @ {}fps, {}s buffer → {}",
                width, height, fps_val, buffer_secs_val, output_path_for_thread
            );
            info!(
                "[recorder] Hotkey: {} (Press to save buffer)",
                hotkey_display
            );

            let mut recorder = create_recorder(
                width,
                height,
                fps_val,
                buffer_secs_val,
                output_path_for_thread.clone(),
            );

            recorder.start().await;

            // Create a channel for hotkey events
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let output_path_for_listener = output_path_for_thread.clone();

            // Spawn the key listener in a blocking task
            tokio::task::spawn_blocking(move || {
                let result = listen(move |event| {
                    // Only process key press events to avoid crashes
                    if let EventType::KeyPress(key) = event.event_type {
                        debug!("[recorder] Key pressed: {:?}", key);

                        // Check if this is our target hotkey
                        if key == target_key {
                            info!(
                                "[recorder] Hotkey {} pressed: sending save signal...",
                                hotkey_display
                            );
                            if let Err(e) = tx.send(output_path_for_listener.clone()) {
                                error!("[recorder] Failed to send save signal: {}", e);
                            }
                        }
                    }
                    // Always continue listening
                });

                match result {
                    Ok(_) => info!("[recorder] Key listener finished normally"),
                    Err(error) => error!("[recorder] Key listener error: {:?}", error),
                }
            });

            // Handle hotkey events in the async context
            while let Some(output_path) = rx.recv().await {
                info!("[recorder] Processing hotkey event: saving buffer...");
                match recorder.save(&output_path) {
                    Ok(()) => {
                        info!("[recorder] ✅ Successfully saved buffer to {}", output_path);
                    }
                    Err(e) => {
                        error!("[recorder] ❌ Failed to save buffer: {}", e);
                    }
                }
                info!("[recorder] Continuing to record...");
            }
        });
    });

    Ok(())
}

fn parse_resolution(resolution: &str) -> anyhow::Result<(u32, u32)> {
    let parts: Vec<&str> = resolution.split('x').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Resolution must be in format WIDTHxHEIGHT (e.g., 1920x1080)"
        ));
    }

    let width = parts[0]
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("Invalid width: {}", parts[0]))?;
    let height = parts[1]
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("Invalid height: {}", parts[1]))?;

    if width < 100 || height < 100 {
        return Err(anyhow::anyhow!("Resolution too small: minimum 100x100"));
    }

    if width > 7680 || height > 4320 {
        return Err(anyhow::anyhow!("Resolution too large: maximum 7680x4320"));
    }

    Ok((width, height))
}
