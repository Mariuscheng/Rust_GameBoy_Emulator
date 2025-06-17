use crate::cpu::InstructionError;
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

    /// 指令錯誤
    #[error("指令錯誤: {0}")]
    Instruction(String),

    /// CPU 指令錯誤
    #[error("CPU 指令錯誤: {0}")]
    InstructionError(#[from] InstructionError),

    /// ROM 相關錯誤
    #[error("ROM 錯誤: {0}")]
    Rom(String),

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

    /// 其他錯誤
    #[error("其他錯誤: {0}")]
    Other(String),

    /// 一般錯誤
    #[error("一般錯誤")]
    Generic(Box<dyn std::error::Error>),
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::Generic(err)
    }
}

/// 模擬器結果類型
pub type Result<T> = std::result::Result<T, Error>;
