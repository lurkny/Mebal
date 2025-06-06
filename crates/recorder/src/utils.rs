use std::process::Command;

pub fn get_video_devices() -> Vec<String> {
    let mut devices = Vec::new();

    devices.push("Desktop (Screen Capture)".to_string());

    if let Ok(output) = Command::new("ffmpeg")
        .args(["-f", "dshow", "-list_devices", "true", "-i", "dummy"])
        .output()
    {
        let stderr = String::from_utf8_lossy(&output.stderr);

        for line in stderr.lines() {
            if line.contains("DirectShow video devices") {
                continue;
            }
            if line.contains("] \"") && line.contains("video") {
                if let Some(start) = line.find("] \"") {
                    if let Some(end) = line[start + 3..].find("\"") {
                        let device_name = &line[start + 3..start + 3 + end];
                        devices.push(device_name.to_string());
                    }
                }
            }
        }
    }

    devices
}

pub fn get_audio_devices() -> Vec<String> {
    let mut devices = Vec::new();

    if let Ok(output) = Command::new("ffmpeg")
        .args(["-f", "dshow", "-list_devices", "true", "-i", "dummy"])
        .output()
    {
        let stderr = String::from_utf8_lossy(&output.stderr);

        for line in stderr.lines() {
            if line.contains("DirectShow audio devices") {
                continue;
            }
            if line.contains("] \"") && line.contains("audio") {
                if let Some(start) = line.find("] \"") {
                    if let Some(end) = line[start + 3..].find("\"") {
                        let device_name = &line[start + 3..start + 3 + end];
                        devices.push(device_name.to_string());
                    }
                }
            }
        }
    }

    devices
}
