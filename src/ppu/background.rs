use super::registers::*;

pub struct Background {
    // 背景圖層的相關數據
    tile_map_base: usize,
    tile_data_base: usize,
}

impl Background {
    pub fn new() -> Self {
        Self {
            tile_map_base: 0x1800,  // 預設使用第一個圖塊映射
            tile_data_base: 0x1000, // 預設使用第二個圖塊數據區
        }
    }

    pub fn update_tile_map(&mut self, lcdc: u8) {
        self.tile_map_base = if lcdc & LCDC_BG_MAP != 0 {
            0x1C00
        } else {
            0x1800
        };
    }

    pub fn update_tile_data(&mut self, lcdc: u8) {
        self.tile_data_base = if lcdc & LCDC_TILE_DATA != 0 {
            0x0000
        } else {
            0x1000
        };
    }

    pub fn render_scanline(&self, vram: &[u8], line: u8, scx: u8, scy: u8) -> [u8; 160] {
        let mut line_buffer = [0u8; 160];

        let y = line.wrapping_add(scy);
        let tile_y = (y / 8) as usize;

        for x in 0..160u8 {
            let scroll_x = x.wrapping_add(scx);
            let tile_x = (scroll_x / 8) as usize;

            // 從瓦片地圖獲取瓦片索引
            let tile_index = vram[self.tile_map_base + tile_y * 32 + tile_x];

            // 計算瓦片內的像素位置
            let px = scroll_x % 8;
            let py = y % 8;

            // 從瓦片數據中獲取像素顏色
            line_buffer[x as usize] = self.get_tile_pixel(vram, tile_index, px, py);
        }

        line_buffer
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
