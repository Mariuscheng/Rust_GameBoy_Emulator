// PPU (Picture Processing Unit) 核心模組
pub mod background; // 背景渲染
pub mod display; // 顯示系統與調色板
pub mod lcd; // LCD 控制器與狀態
pub mod pixel; // 像素和掃描線定義
pub mod ppu; // PPU 主要邏輯
pub mod registers; // PPU 寄存器定義
pub mod sprite; // 精靈渲染
pub mod window; // 視窗渲染

// 重新導出主要組件
pub use background::BackgroundRenderer;
pub use pixel::{create_empty_scanline, map_palette_color_to_rgba, Pixel, ScanLine};
pub use ppu::{PPUMode, PPU};
pub use sprite::SpriteRenderer;
pub use window::WindowRenderer;
