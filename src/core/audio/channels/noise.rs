#[derive(Debug)]
#[allow(dead_code)]
pub struct NoiseChannel {
    enabled: bool,
    length_counter: u8,
    polynomial_counter: u16,
    shift_amount: u8,
    divisor_code: u8,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            polynomial_counter: 0,
            shift_amount: 0,
            divisor_code: 0,
        }
    }
}
