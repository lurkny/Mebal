use std::sync::Arc;
use storage::ReplayBuffer;
use tokio::process::Child;
use tokio::sync::Mutex;


pub struct LinuxRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    child: Option<Child>,
    replay_buffer: Arc<ReplayBuffer>,
    stop_signal: Arc<Mutex<bool>>,
}
