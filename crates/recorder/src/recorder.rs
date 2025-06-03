pub trait Recorder {
    fn start(&mut self);
    fn stop(&mut self);
    fn save(&self, final_output_path: &str);
    fn get_output_path(&self) -> &str; 
}