// Error handling module
pub mod hardware;

use std::fmt;
use thiserror::Error;

// Re-exports
pub use self::hardware::HardwareError;

// Result type definition
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Hardware error: {0}")]
    Hardware(#[from] HardwareError),

    #[error("Instruction error: {0}")]
    Instruction(#[from] InstructionError),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Display error: {0}")]
    Video(String),

    #[error("Memory error: {0}")]
    Memory(String),
}

#[derive(Error, Debug)]
pub enum InstructionError {
    #[error("Invalid opcode: {0:02X}")]
    InvalidOpcode(u8),

    #[error("Invalid register pair: {0:02X}")]
    InvalidRegisterPair(u8),

    #[error("Invalid register: {0:?}")]
    InvalidRegister(RegTarget),

    #[error("Invalid condition: {0:02X}")]
    InvalidCondition(u8),

    #[error("Invalid instruction: {0}")]
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegTarget {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    BC,
    DE,
    HL,
    SP,
    PC,
    AF,
}

impl fmt::Display for RegTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl RegTarget {
    pub fn is_16bit(&self) -> bool {
        matches!(
            self,
            RegTarget::BC
                | RegTarget::DE
                | RegTarget::HL
                | RegTarget::SP
                | RegTarget::PC
                | RegTarget::AF
        )
    }

    pub fn is_8bit(&self) -> bool {
        !self.is_16bit()
    }

    pub fn from_bits(bits: u8) -> Result<Self> {
        match bits & 0x07 {
            0b000 => Ok(RegTarget::B),
            0b001 => Ok(RegTarget::C),
            0b010 => Ok(RegTarget::D),
            0b011 => Ok(RegTarget::E),
            0b100 => Ok(RegTarget::H),
            0b101 => Ok(RegTarget::L),
            0b110 => Ok(RegTarget::HL),
            0b111 => Ok(RegTarget::A),
            _ => Err(Error::Instruction(InstructionError::InvalidRegister(
                RegTarget::A,
            ))),
        }
    }
}
