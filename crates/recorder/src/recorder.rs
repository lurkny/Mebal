#[async_trait::async_trait]
pub trait Recorder: Send + Sync {
    fn new(width: u32, height: u32, fps: u32, buffer_secs: u32, output: String) -> Self
    where
        Self: Sized;
    async fn start(&mut self);
    async fn stop(&mut self);
    fn save(&self, final_output_path: &str) -> Result<(), String>;
    fn get_output_path(&self) -> &str;
}
