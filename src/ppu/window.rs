use crate::error::Result;
use crate::mmu::MMU;
use crate::utils::Logger;
use std::{cell::RefCell, rc::Rc};

use super::background::SCREEN_WIDTH;
use super::pixel::map_palette_color_to_rgba;

// 視窗相關常數
const LCD_CONTROL: u16 = 0xFF40;
const BGP_REGISTER: u16 = 0xFF47;
const WX_REGISTER: u16 = 0xFF4A;
const WY_REGISTER: u16 = 0xFF4B;
const VRAM_TILES_1: u16 = 0x8800;
const VRAM_TILES_2: u16 = 0x8000;
const VRAM_MAPS: u16 = 0x9800;

/// 視窗渲染器
///
/// Game Boy 的視窗是一個可以覆蓋在背景上的獨立圖層。
/// 與背景不同，視窗不能捲動，總是從其指定位置 (WX-7, WY) 開始顯示。
#[derive(Debug)]
pub struct WindowRenderer {
    mmu: Rc<RefCell<MMU>>,
    logger: RefCell<Logger>,
}

impl WindowRenderer {
    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        WindowRenderer {
            mmu,
            logger: RefCell::new(Logger::default()),
        }
    }

    /// 渲染視窗的一條掃描線
    ///
    /// # Arguments
    /// * `line` - 視窗 Y 座標
    /// * `wx` - 視窗 X 座標
    /// * `palette` - 調色板資料
    /// * `display` - 顯示緩衝區
    pub fn render_scanline(&self, line: u8) -> Result<Vec<[u8; 4]>> {
        let mut line_pixels = vec![[255, 255, 255, 255]; SCREEN_WIDTH];
        let mut mmu = self.mmu.borrow_mut();

        // 檢查視窗是否啟用
        let lcdc = mmu.read_byte(0xFF40)?;
        if (lcdc & 0x20) == 0 {
            return Ok(line_pixels);
        }

        // 獲取視窗位置和調色板
        let window_y = mmu.read_byte(0xFF4A)?;
        let window_x = mmu.read_byte(0xFF4B)?.saturating_sub(7);
        let bgp = mmu.read_byte(0xFF47)?;

        if line < window_y {
            return Ok(line_pixels);
        }

        let window_line = line.wrapping_sub(window_y);
        let tile_y = (window_line / 8) as u16;

        // 獲取瓦片數據和地圖的基礎地址
        let tile_data_base = if (lcdc & 0x10) != 0 {
            0x8000u16
        } else {
            0x8800u16
        };

        let map_base = if (lcdc & 0x40) != 0 {
            0x9C00u16
        } else {
            0x9800u16
        };

        for x in 0..SCREEN_WIDTH {
            if (x as u8) < window_x {
                continue;
            }

            let window_x = (x as u8).wrapping_sub(window_x);
            let tile_x = (window_x / 8) as u16;

            // 獲取瓦片編號
            let tile_addr = map_base + tile_y * 32 + tile_x;
            let tile_num = mmu.read_byte(tile_addr)?;

            // 計算瓦片數據地址
            let tile_offset = if (lcdc & 0x10) != 0 {
                (tile_num as u16) * 16
            } else {
                ((tile_num as i8 as i16 + 128) * 16) as u16
            };

            // 獲取瓦片的當前行數據
            let tile_line = (window_line % 8) as u16 * 2;
            let tile_data_addr = tile_data_base + tile_offset + tile_line;

            let tile_data_low = mmu.read_byte(tile_data_addr)?;
            let tile_data_high = mmu.read_byte(tile_data_addr + 1)?;

            // 獲取像素在瓦片中的位置
            let pixel_bit = 7 - (window_x % 8);
            let pixel_low = (tile_data_low >> pixel_bit) & 1;
            let pixel_high = (tile_data_high >> pixel_bit) & 1;
            let color_num = (pixel_high << 1) | pixel_low;

            // 通過調色板獲取最終顏色
            let palette_color = (bgp >> (color_num * 2)) & 0x03;
            line_pixels[x] = map_palette_color_to_rgba(palette_color);
        }

        Ok(line_pixels)
    }

    /// 渲染一條掃描線
    pub fn render_scan_line(
        &self,
        line: u8,
        line_buffer: &mut Vec<[u8; 4]>,
        window_line: u8,
    ) -> Result<()> {
        let mmu = self.mmu.borrow();

        // 讀取所需的寄存器值
        let lcdc = mmu.read_byte(LCD_CONTROL)?;
        let bgp = mmu.read_byte(BGP_REGISTER)?;
        let window_x = mmu.read_byte(WX_REGISTER)?;
        let window_y = mmu.read_byte(WY_REGISTER)?;

        // 確認視窗是否應該被繪製
        if window_y > line {
            return Ok(());
        }

        // 計算視窗的瓦片行
        let tile_y = window_line / 8;
        let tile_fine_y = window_line % 8;

        // 選擇瓦片數據區域
        let tile_data_base = if lcdc & 0x10 != 0 {
            VRAM_TILES_2
        } else {
            VRAM_TILES_1
        };

        // 選擇瓦片地圖
        let map_base = if lcdc & 0x40 != 0 {
            VRAM_MAPS + 0x400
        } else {
            VRAM_MAPS
        };

        // 對每個可見的像素進行渲染
        for x in 0..SCREEN_WIDTH {
            if x < window_x as usize - 7 {
                continue;
            }

            let window_x = x as i16 - (window_x as i16 - 7);
            if window_x < 0 {
                continue;
            }

            // 計算瓦片位置
            let tile_x = (window_x / 8) as u16;
            let tile_fine_x = 7 - (window_x % 8) as u8;

            // 獲取瓦片編號
            let tile_addr = map_base + tile_y as u16 * 32 + tile_x;
            let tile_num = mmu.read_byte(tile_addr)?;

            // 計算瓦片數據地址
            let tile_offset = if lcdc & 0x10 != 0 {
                tile_num as u16 * 16
            } else {
                ((tile_num as i8 as i16 + 128) * 16) as u16
            };

            // 讀取瓦片數據
            let tile_addr = tile_data_base + tile_offset + (tile_fine_y as u16 * 2);
            let tile_low = mmu.read_byte(tile_addr)?;
            let tile_high = mmu.read_byte(tile_addr + 1)?;

            // 計算顏色值
            let color_bit = tile_fine_x;
            let color_num = ((tile_high >> color_bit) & 1) << 1 | ((tile_low >> color_bit) & 1);
            let palette_color = (bgp >> (color_num * 2)) & 0x03;

            // 設置像素顏色
            line_buffer[x] = map_palette_color_to_rgba(palette_color);
        }

        Ok(())
    }
}
