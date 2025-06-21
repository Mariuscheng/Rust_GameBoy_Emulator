#[derive(Debug)]
#[allow(dead_code)]
pub struct Square1Channel {
    enabled: bool,
    length_counter: u8,
    frequency: u16,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
}

impl Square1Channel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            frequency: 0,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
        }
    }
}
