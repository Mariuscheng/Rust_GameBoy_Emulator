//! Display 模組，管理調色板與 framebuffer

use super::background::BackgroundRenderer;

pub struct Display {
    pub framebuffer: Vec<u32>,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
}

impl Display {
    pub fn new() -> Self {
        Self {
            framebuffer: vec![0xFFFFFFFF; 160 * 144],
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
        }
    }
    pub fn map_color(&self, palette: u8, color_id: u8) -> u8 {
        (palette >> (color_id * 2)) & 0x03
    }
    pub fn clear(&mut self) {
        self.framebuffer.fill(0xFFFFFFFF);
    }
    pub fn render_game_frame(&mut self, bg: &BackgroundRenderer, mmu: &crate::mmu::MMU) {
        for y in 0..144u8 {
            let line = bg.render_line(y, mmu);
            for x in 0..160u8 {
                let color_id = line[x as usize];
                // 依 Game Boy 調色板轉換為灰階顏色
                let gray = match color_id {
                    0 => 0xFFFFFFFF,
                    1 => 0xFFAAAAAA,
                    2 => 0xFF555555,
                    _ => 0xFF000000,
                };
                self.framebuffer[(y as usize) * 160 + (x as usize)] = gray;
            }
        }
    }
}
