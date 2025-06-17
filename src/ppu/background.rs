use crate::error::Result;
use crate::mmu::MMU;
use std::{cell::RefCell, rc::Rc};

use super::pixel::map_palette_color_to_rgba;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

// VRAM 相關常數
pub const VRAM_TILE_DATA_0: u16 = 0x8000;
pub const VRAM_TILE_DATA_1: u16 = 0x8800;
pub const VRAM_MAP_0: u16 = 0x9800;
pub const VRAM_MAP_1: u16 = 0x9C00;

// LCD 控制位元
pub const LCDC_BG_ENABLE: u8 = 0x01;
pub const LCDC_TILE_DATA: u8 = 0x10;
pub const LCDC_BG_MAP: u8 = 0x08;

// LCD 相關暫存器位址
pub const LCD_CONTROL: u16 = 0xFF40;
pub const SCROLL_Y: u16 = 0xFF42;
pub const SCROLL_X: u16 = 0xFF43;
pub const BGP: u16 = 0xFF47; // 背景調色板

#[derive(Debug)]
pub struct BackgroundRenderer {
    mmu: Rc<RefCell<MMU>>,
}

impl BackgroundRenderer {
    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        BackgroundRenderer { mmu }
    }

    pub fn render_scanline(&self, line: u8) -> Result<Vec<[u8; 4]>> {
        let mut line_pixels = vec![[255, 255, 255, 255]; SCREEN_WIDTH];
        let mmu = self.mmu.borrow();

        // 讀取控制寄存器
        let lcdc = mmu.read_byte(LCD_CONTROL)?;
        if lcdc & LCDC_BG_ENABLE == 0 {
            return Ok(line_pixels);
        }

        let scroll_y = mmu.read_byte(SCROLL_Y)?;
        let scroll_x = mmu.read_byte(SCROLL_X)?;
        let bgp = mmu.read_byte(BGP)?;

        // 計算在瓦片地圖中的位置
        let y_pos = line.wrapping_add(scroll_y);
        let tile_row = (y_pos / 8) as u16;

        // 決定使用哪個瓦片數據區域和地圖
        let tile_data_base = if lcdc & LCDC_TILE_DATA != 0 {
            VRAM_TILE_DATA_0
        } else {
            VRAM_TILE_DATA_1
        };

        let map_base = if lcdc & LCDC_BG_MAP != 0 {
            VRAM_MAP_1
        } else {
            VRAM_MAP_0
        };

        // 針對掃描線上的每個像素進行渲染
        for x in 0..SCREEN_WIDTH {
            let x_pos = (x as u8).wrapping_add(scroll_x);
            let tile_col = (x_pos / 8) as u16;

            // 獲取瓦片編號
            let tile_addr = map_base + tile_row * 32 + tile_col;
            let tile_num = mmu.read_byte(tile_addr)?;

            // 計算瓦片數據地址
            let tile_offset = if lcdc & LCDC_TILE_DATA != 0 {
                (tile_num as u16) * 16
            } else {
                ((tile_num as i8 as i16 + 128) * 16) as u16
            };

            // 獲取瓦片的當前行數據
            let tile_line = (y_pos % 8) as u16 * 2;
            let tile_data_addr = tile_data_base + tile_offset + tile_line;

            let tile_data_low = mmu.read_byte(tile_data_addr)?;
            let tile_data_high = mmu.read_byte(tile_data_addr + 1)?;

            // 獲取像素在瓦片中的位置
            let pixel_bit = 7 - (x_pos % 8);
            let pixel_low = (tile_data_low >> pixel_bit) & 1;
            let pixel_high = (tile_data_high >> pixel_bit) & 1;
            let color_num = (pixel_high << 1) | pixel_low;

            // 通過調色板獲取最終顏色
            let palette_color = (bgp >> (color_num * 2)) & 0x03;
            line_pixels[x] = map_palette_color_to_rgba(palette_color);
        }

        Ok(line_pixels)
    }
}
