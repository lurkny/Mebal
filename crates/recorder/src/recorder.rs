pub trait Recorder {
    fn start(&mut self);
    fn stop(&mut self);
    fn save(&self, path: &str);
}
