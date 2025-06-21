use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Hardware error: {0}")]
    Hardware(#[from] HardwareError),
    
    #[error("Instruction error: {0}")]
    Instruction(#[from] InstructionError),
    
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum HardwareError {
    #[error("Invalid memory address: {0:04X}")]
    InvalidAddress(u16),
    
    #[error("Memory controller error: {0}")]
    MemoryController(String),
    
    #[error("PPU error: {0}")]
    PPU(String),
    
    #[error("Timer error: {0}")]
    Timer(String),
}

#[derive(Error, Debug)]
pub enum InstructionError {
    #[error("Invalid opcode: {0:02X}")]
    InvalidOpcode(u8),
    
    #[error("Invalid register: {0:02X}")]
    InvalidRegister(u8),
    
    #[error("Invalid register pair: {0:02X}")]
    InvalidRegisterPair(u8),
    
    #[error("Custom error: {0}")]
    Custom(String),
}