use std::thread;

use dioxus::{desktop::{Config, WindowBuilder}, prelude::*};
use rdev::{listen, EventType, Key};
use recorder::create_recorder;
use env_logger;
use log::{info, debug};

static CSS: Asset = asset!("/assets/main.css");

#[derive(PartialEq, Debug, Clone)]
struct RecordingConfig {
    resolution: Signal<String>,
    fps: Signal<String>,
    output_path: Signal<String>,
    buffer_secs: Signal<String>,
    listener_started: Signal<bool>,
}
impl RecordingConfig {
    fn new(
        resolution: Signal<String>,
        fps: Signal<String>,
        output_path: Signal<String>,
        buffer_secs: Signal<String>,
        listener_started: Signal<bool>,
    ) -> Self {
        Self {
            resolution,
            fps,
            output_path,
            buffer_secs,
            listener_started,
        }
    }
}

fn main() {
    env_logger::init();
    

    let d_cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("Mebal")
            .with_decorations(true)
    )
    .with_background_color((255, 255, 255, 255)) // White background
    .with_disable_context_menu(true);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(d_cfg)
        .launch(app);
}

pub fn app() -> Element {
    let _config = use_context_provider( || RecordingConfig::new(
        Signal::new("1920x1080".to_string()),
        Signal::new("30".to_string()),
        Signal::new("/path/to/output.mp4".to_string()),
        Signal::new("10".to_string()),
        Signal::new(false),
    ));

    rsx! {
            document::Stylesheet { href: CSS }
            ResolutionInput {}
            FpsInput {  }
            OutputPathInput {  }
            BufferSecondsInput {  }
            StartBufferButton {  }
        
    }
}

#[component]
fn ResolutionInput() -> Element {
    let mut res = use_context::<RecordingConfig>().resolution;
    rsx! {
        div { class: "form-group",
            label { "Resolution" }
            input {
                value: "{res}",
                oninput: move |e| res.set(e.value().clone()),
                placeholder: "e.g., 1920x1080"
            }
        }
    }
}

#[component]
fn FpsInput() -> Element {
    let mut fps = use_context::<RecordingConfig>().fps;
    rsx! {
        div { class: "form-group",
            label { "FPS" }
            input {
                value: "{fps}",
                oninput: move |e| fps.set(e.value().clone()),
                placeholder: "e.g., 30"
            }
        }
    }
}

#[component]
fn OutputPathInput() -> Element {
    let mut output_path = use_context::<RecordingConfig>().output_path;
    rsx! {
        div { class: "form-group",
            label { "Output Path" }
            input {
                value: "{output_path}",
                oninput: move |e| output_path.set(e.value().clone()),
                placeholder: "/path/to/output.mp4"
            }
        }
    }
}

#[component]
fn BufferSecondsInput() -> Element {
    let mut buffer_secs = use_context::<RecordingConfig>().buffer_secs;
    rsx! {
        div { class: "form-group",
            label { "Buffer Seconds" }
            input {
                value: "{buffer_secs}",
                oninput: move |e| buffer_secs.set(e.value().clone()),
                placeholder: "e.g., 10"
            }
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
    rsx! {
        button {
            onclick: move |_| {
                if !*listener_started.read() {
                    let resolution = resolution_sig.read().clone();
                    let fps = fps_sig.read().clone();
                    let output_path = output_path_sig.read().clone();
                    let buffer_secs = buffer_secs_sig.read().clone();
                    start_recording(&resolution, &fps, &output_path, &buffer_secs);
                    listener_started.set(true);
                }
            },
            "Start Buffer"
        }
    }
}

fn start_recording(resolution: &str, fps: &str, output_path: &str, buffer_secs: &str) {
    // clone all inputs into owned strings for the spawned thread
    let resolution = resolution.to_string();
    let fps = fps.to_string();
    let buffer_secs = buffer_secs.to_string();
    let output_path = output_path.to_string();

    thread::spawn(move ||  {
        // parse parameters
        let (w, h) = resolution.split_once('x').unwrap();
        let width = w.parse::<u32>().unwrap();
        let height = h.parse::<u32>().unwrap();
        let fps = fps.parse::<u32>().unwrap();
        let buffer_secs = buffer_secs.parse::<u32>().unwrap();

        // create & start ffmpeg recorder
        info!("[recorder] Starting: {}x{} @ {}fps, {}s buffer → {}", width, height, fps, buffer_secs, output_path);
        let mut recorder = create_recorder(width, height, fps, buffer_secs, output_path.clone());
        recorder.start();

        // now block this thread on F2
        let result = listen(move |event| {
            // debug: log every key press
            if let EventType::KeyPress(key) = event.event_type {
                debug!("[recorder] key press event: {:?}", key);
            }
            if let EventType::KeyPress(Key::F2) = event.event_type {
                info!("[recorder] F2 pressed: stopping");
                recorder.stop();
                recorder.save(&output_path);
                info!("[recorder] Saved output to {}", output_path);
                return;
            }
            return;
        });
        debug!("[recorder] Listener exited with: {:?}", result);
    });
}
