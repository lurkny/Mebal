

pub trait Recorder {
    fn start(&self);
    fn stop(&self);
    fn save(&self, path: &str);
}

