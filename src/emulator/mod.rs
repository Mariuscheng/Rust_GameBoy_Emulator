// Game Boy Emulator 整合入口
// 匯集 CPU、PPU、APU、MMU、Timer、Joypad

pub mod core;

// 重新導出主要 Emulator 結構
pub use core::Emulator;
