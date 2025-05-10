use dioxus::prelude::*;

static CSS: Asset = asset!("/assets/main.css");


#[derive(PartialEq, Debug)]
struct Config {
    resolution_width: String,
    resolution_height: String,
    fps: String,
    output_path: String,
}

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut resolution = use_signal( || String::from("1920x1080"));
    let mut fps = use_signal( || String::from("30"));
    let mut output_path = use_signal( || String::from("/path/to/output.mp4"));

    rsx!{
        document::Stylesheet { href: CSS }
        div {
            class: "form-group",
            label { "Resolution" }
            input {
                value: "{resolution}",
                oninput: move |e| resolution.set(e.value().clone()),
                placeholder: "e.g., 1920x1080"
            }
        },
        div {
            class: "form-group",
            label { "FPS" }
            input {
                value: "{fps}",
                oninput: move |e| fps.set(e.value().clone()),
                placeholder: "e.g., 30"
            }
        },
        div {
            class: "form-group",
            label { "Output Path" }
            input {
                value: "{output_path}",
                oninput: move |e| output_path.set(e.value().clone()),
                placeholder: "/path/to/output.mp4"
            }
        },
        button {
            onclick: move |_| {
               // println!("Starting recording with resolution: {}, fps: {}, output path: {}", resolution.get(), fps.get(), output_path.get());
                // Insert recording logic here
            },
            "Start Recording"
        }
    }
}