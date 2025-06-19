//! 背景渲染器，產生背景圖層像素

use crate::mmu::MMU;
use crate::ppu::registers::{BGP, LCDC, SCX, SCY};

pub struct BackgroundRenderer;

impl BackgroundRenderer {
    /// 產生一條掃描線的背景像素顏色 ID（0~3），需傳入 MMU 以讀取 VRAM/暫存器
    pub fn render_line(&self, line: u8, mmu: &MMU) -> Vec<u8> {
        let mut result = vec![0u8; 160];
        // 讀取 PPU 相關暫存器
        let lcdc = mmu.read_byte(LCDC).unwrap_or(0x91);
        let scx = mmu.read_byte(SCX).unwrap_or(0);
        let scy = mmu.read_byte(SCY).unwrap_or(0);
        let bgp = mmu.read_byte(BGP).unwrap_or(0xFC);
        let vram = mmu.vram();

        // 背景啟用位
        if (lcdc & 0x01) == 0 {
            return result; // 全部顏色 0（白）
        }
        // 決定 tile map 起始位址
        let bg_tile_map = if (lcdc & 0x08) != 0 { 0x1C00 } else { 0x1800 };
        // 決定 tile data 區塊
        let tile_data_base = if (lcdc & 0x10) != 0 { 0x0000 } else { 0x1000 };
        for x in 0..160u8 {
            let x_pos = (x as u16 + scx as u16) & 0xFF;
            let y_pos = (line as u16 + scy as u16) & 0xFF;
            let tile_x = (x_pos / 8) as usize;
            let tile_y = (y_pos / 8) as usize;
            let tile_map_index = bg_tile_map + tile_y * 32 + tile_x;
            let tile_index = vram.get(tile_map_index).copied().unwrap_or(0);
            // 計算 tile data 位址
            let tile_addr = if (lcdc & 0x10) != 0 {
                tile_data_base + (tile_index as u16 * 16)
            } else {
                tile_data_base + ((tile_index as i8 as i16 + 128) as u16 * 16)
            };
            let py = (y_pos % 8) as usize;
            let px = (x_pos % 8) as usize;
            let byte1 = vram.get(tile_addr as usize + py * 2).copied().unwrap_or(0);
            let byte2 = vram
                .get(tile_addr as usize + py * 2 + 1)
                .copied()
                .unwrap_or(0);
            let color_bit = 7 - px;
            let color_num = ((byte2 >> color_bit) & 1) << 1 | ((byte1 >> color_bit) & 1);
            // 轉換 palette
            let palette_color = (bgp >> (color_num * 2)) & 0x03;
            result[x as usize] = palette_color;
        }
        result
    }
}
