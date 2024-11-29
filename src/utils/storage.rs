use ringbuf::{traits::*, HeapRb};


pub struct FrameBuffer {
   pub buffer: HeapRb<Vec<u8>>,
    width: u32,
    height: u32,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32, recording_time: usize, frame_rate: usize) -> Self {
        let buffer = HeapRb::new(calculate_buffer_size(&recording_time, &frame_rate));
        Self { buffer, width, height }
    }


    pub fn push(&mut self, frame: Vec<u8>) {
        if let Err(e) = self.buffer.try_push(frame.clone()){
            println!("Buffer full, dropping frame");
            self.buffer.try_pop();
            self.buffer.try_push(frame);
        }
    }

    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.buffer.try_pop()
    }

}

fn calculate_buffer_size( recording_time: &usize, frame_rate: &usize) -> usize {
    (frame_rate * recording_time) as usize
}