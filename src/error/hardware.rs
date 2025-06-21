use thiserror::Error;

#[derive(Error, Debug)]
pub enum HardwareError {
    #[error("Memory mapping error: {0}")]
    MemoryMap(String),

    #[error("Memory read error: {0}")]
    MemoryRead(String),

    #[error("Memory write error: {0}")]
    MemoryWrite(String),

    #[error("Interrupt error: {0}")]
    Interrupt(String),

    #[error("Timer error: {0}")]
    Timer(String),

    #[error("PPU error: {0}")]
    PPU(String),

    #[error("APU error: {0}")]
    APU(String),

    #[error("Joypad error: {0}")]
    Joypad(String),

    #[error("DMA transfer error: {0}")]
    DMA(String),

    #[error("Custom hardware error: {0}")]
    Custom(String),

    #[error("Display error: {0}")]
    Display(String),

    #[error("Audio error: {0}")]
    Audio(String),
}

impl HardwareError {
    pub fn memory_map(msg: impl Into<String>) -> Self {
        HardwareError::MemoryMap(msg.into())
    }

    pub fn memory_read(msg: impl Into<String>) -> Self {
        HardwareError::MemoryRead(msg.into())
    }

    pub fn memory_write(msg: impl Into<String>) -> Self {
        HardwareError::MemoryWrite(msg.into())
    }

    pub fn interrupt(msg: impl Into<String>) -> Self {
        HardwareError::Interrupt(msg.into())
    }

    pub fn timer(msg: impl Into<String>) -> Self {
        HardwareError::Timer(msg.into())
    }

    pub fn ppu(msg: impl Into<String>) -> Self {
        HardwareError::PPU(msg.into())
    }

    pub fn apu(msg: impl Into<String>) -> Self {
        HardwareError::APU(msg.into())
    }

    pub fn joypad(msg: impl Into<String>) -> Self {
        HardwareError::Joypad(msg.into())
    }

    pub fn dma(msg: impl Into<String>) -> Self {
        HardwareError::DMA(msg.into())
    }

    pub fn custom(msg: impl Into<String>) -> Self {
        HardwareError::Custom(msg.into())
    }

    pub fn with_address(self, address: u16) -> String {
        format!("{} [address: 0x{:04X}]", self, address)
    }

    pub fn with_context(self, context: impl Into<String>) -> String {
        format!("{} ({})", self, context.into())
    }

    pub fn is_memory_error(&self) -> bool {
        matches!(
            self,
            HardwareError::MemoryMap(_)
                | HardwareError::MemoryRead(_)
                | HardwareError::MemoryWrite(_)
        )
    }

    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            HardwareError::MemoryMap(_) | HardwareError::DMA(_) | HardwareError::Custom(_)
        )
    }
}

impl From<String> for HardwareError {
    fn from(s: String) -> Self {
        HardwareError::Custom(s)
    }
}

impl From<&str> for HardwareError {
    fn from(s: &str) -> Self {
        HardwareError::Custom(s.to_string())
    }
}

/// ROM related errors
#[derive(Error, Debug)]
pub enum ROMError {
    #[error("Invalid ROM size: {0}")]
    InvalidSize(usize),

    #[error("Invalid cartridge type: {0}")]
    InvalidCartridgeType(u8),

    #[error("ROM checksum error")]
    ChecksumMismatch,

    #[error("ROM loading failed: {0}")]
    LoadError(String),

    #[error("Unsupported MBC type: {0}")]
    UnsupportedMBC(u8),
}
