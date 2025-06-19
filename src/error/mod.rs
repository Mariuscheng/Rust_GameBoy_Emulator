use crate::cpu::instructions::common::InstructionError;
use std::io;
use thiserror::Error;

/// Game Boy 模擬器的錯誤類型
#[derive(Error, Debug)]
pub enum Error {
    /// IO 錯誤
    #[error("IO 錯誤: {0}")]
    IO(#[from] io::Error),
    /// 記憶體錯誤
    #[error("記憶體錯誤: {0}")]
    Memory(String),
    /// CPU 指令錯誤
    #[error("CPU 指令錯誤: {0}")]
    Instruction(#[from] InstructionError),
    /// 無效的記憶體地址
    #[error("無效的記憶體地址: {0:04X}")]
    InvalidAddress(u16),
    /// ROM 相關錯誤
    #[error("ROM 錯誤: {0}")]
    Rom(String),
    /// ROM 驗證錯誤
    #[error("ROM 驗證錯誤: {0}")]
    ROMValidation(String),
    /// VRAM 不可訪問
    #[error("VRAM 當前不可訪問")]
    VramInaccessible,
    /// OAM 不可訪問
    #[error("OAM 當前不可訪問")]
    OamInaccessible,
    /// LCD 已停用
    #[error("LCD 已停用")]
    LcdDisabled,
    /// Timer 中斷失敗
    #[error("Timer 中斷請求失敗")]
    TimerInterruptFailed,
    /// PPU 相關錯誤
    #[error("PPU 錯誤: {0}")]
    PPU(String),
    /// APU 相關錯誤
    #[error("APU 錯誤: {0}")]
    APU(String),
    /// 通用錯誤
    #[error("{0}")]
    Generic(String),
    /// 其他錯誤
    #[error("{0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error + 'static>> for Error {
    fn from(err: Box<dyn std::error::Error + 'static>) -> Self {
        Error::Generic(err.to_string())
    }
}

impl Error {
    /// 創建一個新的錯誤實例
    pub fn new(msg: &str) -> Self {
        Error::Other(msg.to_string())
    }
}

/// 模擬器結果類型
pub type Result<T> = std::result::Result<T, Error>;
