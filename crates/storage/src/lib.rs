use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct TimestampedPacket {
    pub data: Vec<u8>,
    pub timestamp: Instant,
    pub is_keyframe: bool,
}

pub struct ReplayBuffer {
    packets: Arc<Mutex<VecDeque<TimestampedPacket>>>,
    max_duration: Duration,
    max_size_bytes: usize,
    current_size: Arc<Mutex<usize>>,
}

impl ReplayBuffer {
    pub fn new(buffer_duration_secs: u32, max_size_mb: usize) -> Self {
        Self {
            packets: Arc::new(Mutex::new(VecDeque::new())),
            max_duration: Duration::from_secs(buffer_duration_secs as u64),
            max_size_bytes: max_size_mb * 1024 * 1024,
            current_size: Arc::new(Mutex::new(0)),
        }
    }

    pub fn add_packet(&self, data: Vec<u8>, is_keyframe: bool) {
        let packet = TimestampedPacket {
            data,
            timestamp: Instant::now(),
            is_keyframe,
        };

        let mut packets = self.packets.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();

        *current_size += packet.data.len();
        packets.push_back(packet);

        let cutoff_time = Instant::now() - self.max_duration;
        while let Some(front) = packets.front() {
            if front.timestamp < cutoff_time || *current_size > self.max_size_bytes {
                let removed = packets.pop_front().unwrap();
                *current_size -= removed.data.len();
            } else {
                break;
            }
        }
    }

    pub fn save_to_file(&self, output_path: &str) -> anyhow::Result<()> {
        let packets = self.packets.lock().unwrap();

        let start_index = packets.iter().position(|p| p.is_keyframe).unwrap_or(0);

        // Write packets to temporary file, then use ffmpeg to mux
        let temp_path = std::env::temp_dir().join("replay_temp.h264");
        let mut file = std::fs::File::create(&temp_path)?;

        for packet in packets.iter().skip(start_index) {
            use std::io::Write;
            file.write_all(&packet.data)?;
        }

        // Use ffmpeg to convert raw H.264 to MP4
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "h264",
                "-i",
                temp_path.to_str().unwrap(),
                "-c",
                "copy",
                output_path,
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg conversion failed"));
        }

        // Clean up temp file
        std::fs::remove_file(temp_path)?;
        Ok(())
    }
}

/// H.264 NAL unit parser for keyframe detection
pub struct H264Parser {
    buffer: Vec<u8>,
}

impl H264Parser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Process incoming data and extract complete NAL units
    pub fn process_data(&mut self, data: &[u8]) -> Vec<(Vec<u8>, bool)> {
        self.buffer.extend_from_slice(data);
        let mut packets = Vec::new();

        // Look for NAL unit start codes (0x00 0x00 0x00 0x01 or 0x00 0x00 0x01)
        let mut start = 0;
        while start < self.buffer.len() {
            if let Some(next_start) = self.find_next_nal_start(start + 1) {
                // Found complete NAL unit
                let nal_data = self.buffer[start..next_start].to_vec();
                let is_keyframe = self.is_keyframe_nal(&nal_data);
                packets.push((nal_data, is_keyframe));
                start = next_start;
            } else {
                // Incomplete NAL unit, keep remaining data for next call
                self.buffer.drain(0..start);
                break;
            }
        }

        packets
    }

    fn find_next_nal_start(&self, start: usize) -> Option<usize> {
        if start + 3 >= self.buffer.len() {
            return None;
        }

        for i in start..self.buffer.len() - 3 {
            // Check for 4-byte start code (0x00 0x00 0x00 0x01)
            if self.buffer[i] == 0x00
                && self.buffer[i + 1] == 0x00
                && self.buffer[i + 2] == 0x00
                && self.buffer[i + 3] == 0x01
            {
                return Some(i);
            }
            // Check for 3-byte start code (0x00 0x00 0x01)
            if i > 0
                && self.buffer[i] == 0x00
                && self.buffer[i + 1] == 0x00
                && self.buffer[i + 2] == 0x01
            {
                return Some(i);
            }
        }
        None
    }

    fn is_keyframe_nal(&self, nal_data: &[u8]) -> bool {
        // Skip start code to get to NAL header
        let nal_start = if nal_data.len() >= 4 && nal_data[0..4] == [0x00, 0x00, 0x00, 0x01] {
            4
        } else if nal_data.len() >= 3 && nal_data[0..3] == [0x00, 0x00, 0x01] {
            3
        } else {
            return false;
        };

        if nal_start >= nal_data.len() {
            return false;
        }

        // NAL unit type is in the lower 5 bits of the first byte after start code
        let nal_type = nal_data[nal_start] & 0x1F;

        // Type 5 = IDR slice (keyframe)
        // Type 7 = SPS (Sequence Parameter Set)
        // Type 8 = PPS (Picture Parameter Set)
        matches!(nal_type, 5 | 7 | 8)
    }
}
