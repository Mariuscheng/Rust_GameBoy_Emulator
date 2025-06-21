#[derive(Debug)]
#[allow(dead_code)]
pub struct WaveChannel {
    enabled: bool,
    length_counter: u8,
    frequency: u16,
    volume: u8,
    pattern: [u8; 16],
}

impl WaveChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            frequency: 0,
            volume: 0,
            pattern: [0; 16],
        }
    }
}
