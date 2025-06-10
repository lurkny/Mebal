use async_trait::async_trait;
use common::log::{error, info, warn};
use std::process::Stdio;
use std::sync::Arc;
use storage::ReplayBuffer;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::recorder::Recorder;

pub struct OsxRecorder {
    width: u32,
    height: u32,
    fps: u32,
    buffer_secs: u32,
    output: String,
    child: Option<Child>,
    replay_buffer: Arc<ReplayBuffer>,
    stop_signal: Arc<Mutex<bool>>,
}

impl OsxRecorder {}
