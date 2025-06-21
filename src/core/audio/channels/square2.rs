#[derive(Debug)]
#[allow(dead_code)]
pub struct Square2Channel {
    enabled: bool,
    length_counter: u8,
    frequency: u16,
    duty_cycle: u8,
}

impl Square2Channel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            frequency: 0,
            duty_cycle: 0,
        }
    }
}
