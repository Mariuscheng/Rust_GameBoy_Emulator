//! LCD 控制器模組，管理 LCD 狀態與暫存器

#[derive(Default)]
pub struct LCDController {
    pub lcdc: u8, // LCD 控制暫存器
    pub stat: u8, // LCD 狀態暫存器
    pub ly: u8,   // 掃描線
    pub lyc: u8,  // 掃描線比較
}

impl LCDController {
    pub fn update(&mut self) {
        // 狀態機邏輯（略）
    }
    pub fn is_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }
}
