pub trait Recorder {
    fn start(&mut self);
    fn stop(&mut self);
    fn save(&self, final_output_path: &str); // final_output_path is the argument from the main F2 press
    fn get_output_path(&self) -> &str; // Added to provide the configured output path
}
