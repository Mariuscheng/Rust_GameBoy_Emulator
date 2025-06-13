use super::registers::*;

pub struct Window {
    tile_map_base: usize,
    tile_data_base: usize,
    internal_line: u8,  // 視窗內部的行計數器
}

impl Window {
    pub fn new() -> Self {
        Self {
            tile_map_base: 0x1800,  // 預設使用第一個圖塊映射
            tile_data_base: 0x1000, // 預設使用第二個圖塊數據區
            internal_line: 0,
        }
    }

    pub fn update_tile_map(&mut self, lcdc: u8) {
        self.tile_map_base = if lcdc & LCDC_WIN_MAP != 0 { 0x1C00 } else { 0x1800 };
    }

    pub fn update_tile_data(&mut self, lcdc: u8) {
        self.tile_data_base = if lcdc & LCDC_TILE_DATA != 0 { 0x0000 } else { 0x1000 };
    }

    pub fn render_scanline(&mut self, vram: &[u8], line: u8, wx: u8, wy: u8) -> Option<[u8; 160]> {
        if line < wy {
            return None;
        }

        let mut line_buffer = [0u8; 160];
        let effective_wx = wx.wrapping_sub(7);

        if effective_wx >= 160 {
            return None;
        }

        let tile_y = (self.internal_line / 8) as usize;

        for x in effective_wx..160u8 {
            let window_x = x.wrapping_sub(effective_wx);
            let tile_x = (window_x / 8) as usize;
            
            // 從瓦片地圖獲取瓦片索引
            let map_addr = self.tile_map_base + tile_y * 32 + tile_x;
            let tile_index = vram[map_addr];
            
            // 計算瓦片內的像素位置
            let px = window_x % 8;
            let py = self.internal_line % 8;
            
            // 從瓦片數據中獲取像素顏色
            line_buffer[x as usize] = self.get_tile_pixel(vram, tile_index, px, py);
        }

        self.internal_line = self.internal_line.wrapping_add(1);
        Some(line_buffer)
    }

    pub fn reset_line_counter(&mut self) {
        self.internal_line = 0;
    }

    fn get_tile_pixel(&self, vram: &[u8], tile_index: u8, px: u8, py: u8) -> u8 {
        let tile_addr = if self.tile_data_base == 0x0000 {
            self.tile_data_base + (tile_index as usize * 16)
        } else {
            self.tile_data_base + ((tile_index as i8 as i16 + 128) as usize * 16)
        };

        let low_byte = vram[tile_addr + (py as usize * 2)];
        let high_byte = vram[tile_addr + (py as usize * 2) + 1];
        
        let shift = 7 - px;
        let low_bit = (low_byte >> shift) & 1;
        let high_bit = (high_byte >> shift) & 1;
        
        (high_bit << 1) | low_bit
    }
}
