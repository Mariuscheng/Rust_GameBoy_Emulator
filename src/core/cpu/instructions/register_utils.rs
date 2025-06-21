use crate::error::{Error, InstructionError, RegTarget, Result};

// Convert register pair bits to register pair targets
pub fn get_reg_pair(bits: u8) -> Result<(RegTarget, RegTarget)> {
    match bits & 0x0F {
        0x00 => Ok((RegTarget::B, RegTarget::C)),
        0x01 => Ok((RegTarget::D, RegTarget::E)),
        0x02 => Ok((RegTarget::H, RegTarget::L)),
        0x03 => Ok((RegTarget::SP, RegTarget::SP)),
        _ => Err(Error::Instruction(InstructionError::InvalidRegisterPair(
            bits,
        ))),
    }
}

// Convert bit pattern to single register target
pub fn get_reg_target(bits: u8) -> Result<RegTarget> {
    match bits & 0x07 {
        0x00 => Ok(RegTarget::B),
        0x01 => Ok(RegTarget::C),
        0x02 => Ok(RegTarget::D),
        0x03 => Ok(RegTarget::E),
        0x04 => Ok(RegTarget::H),
        0x05 => Ok(RegTarget::L),
        0x06 => Ok(RegTarget::HL),
        0x07 => Ok(RegTarget::A),
        _ => Err(Error::Instruction(InstructionError::InvalidRegister(
            RegTarget::A,
        ))),
    }
}

// 標誌位操作 trait
pub trait FlagOperations {
    fn set_zero(&mut self, value: bool);
    fn set_subtract(&mut self, value: bool);
    fn set_half_carry(&mut self, value: bool);
    fn set_carry(&mut self, value: bool);
    fn get_zero(&self) -> bool;
    fn get_subtract(&self) -> bool;
    fn get_half_carry(&self) -> bool;
    fn get_carry(&self) -> bool;
}

// 計算 16 位元加法的進位
pub fn calc_16bit_carry(a: u16, b: u16, c: bool) -> bool {
    let c_in = if c { 1 } else { 0 };
    let result = (a as u32) + (b as u32) + (c_in as u32);
    result > 0xFFFF
}

// 計算 8 位元加法的半進位
pub fn calc_half_carry(a: u8, b: u8, c: bool) -> bool {
    let c_in = if c { 1 } else { 0 };
    let result = (a & 0x0F) + (b & 0x0F) + c_in;
    result > 0x0F
}

// 計算 16 位元加法的半進位
pub fn calc_16bit_half_carry(a: u16, b: u16) -> bool {
    ((a & 0x0FFF) + (b & 0x0FFF)) > 0x0FFF
}
