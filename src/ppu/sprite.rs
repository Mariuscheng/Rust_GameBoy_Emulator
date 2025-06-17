use super::{
    display::Display,
    pixel::{map_palette_color_to_rgba, Pixel},
};
use crate::error::Result;
use crate::mmu::MMU;
use crate::utils::Logger;
use std::{cell::RefCell, rc::Rc};

// LCD 控制寄存器位
const LCD_CONTROL: u16 = 0xFF40;
const LCDC_SPRITE_ENABLE: u8 = 0x02;
const LCDC_SPRITE_SIZE: u8 = 0x04;

// 調色板寄存器
const OBP0: u16 = 0xFF48; // 對象調色板 0
const OBP1: u16 = 0xFF49; // 對象調色板 1

/// 表示一個 Game Boy 精靈（OBJ）
#[derive(Debug, Clone)]
pub struct Sprite {
    pub x: i16,      // X 座標 (負值表示部分可見)
    pub y: i16,      // Y 座標 (負值表示部分可見)
    pub tile_id: u8, // 圖塊編號
    pub flags: u8,   // 精靈屬性標誌
}

impl Sprite {
    /// 從 OAM 數據創建新的精靈
    ///
    /// # Arguments
    /// * `oam_data` - 4 bytes of OAM data:
    ///   - byte 0: Y 位置 (顯示位置 = 值-16)
    ///   - byte 1: X 位置 (顯示位置 = 值-8)
    ///   - byte 2: 圖塊編號
    ///   - byte 3: 屬性/標誌:
    ///     - bit 7: 背景和視窗優先級 (0=精靈在上面, 1=精靈在下面)
    ///     - bit 6: Y 軸翻轉     (0=正常, 1=垂直翻轉)
    ///     - bit 5: X 軸翻轉     (0=正常, 1=水平翻轉)
    ///     - bit 4: 調色板編號   (0=OBP0, 1=OBP1)
    pub fn new(oam_data: &[u8]) -> Self {
        Sprite {
            y: (oam_data[0] as i16) - 16,
            x: (oam_data[1] as i16) - 8,
            tile_id: oam_data[2],
            flags: oam_data[3],
        }
    }

    /// 檢查精靈是否在可見區域內
    pub fn is_visible(&self) -> bool {
        self.x > -8 && self.x < 160 && self.y > -16 && self.y < 144
    }

    /// 獲取精靈的調色板編號 (0 = OBP0, 1 = OBP1)
    pub fn palette(&self) -> bool {
        (self.flags & 0x10) != 0
    }

    /// 檢查精靈是否在背景和視窗後面
    pub fn behind_background(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    /// 檢查精靈是否水平翻轉
    pub fn flip_x(&self) -> bool {
        (self.flags & 0x20) != 0
    }

    /// 檢查精靈是否垂直翻轉
    pub fn flip_y(&self) -> bool {
        (self.flags & 0x40) != 0
    }
}

/// 表示一個精靈像素的資訊
#[derive(Debug, Clone, Copy)]
pub struct SpritePixel {
    pub color_id: u8,    // 顏色 ID (0-3)
    pub palette: bool,   // false = OBP0, true = OBP1
    pub behind_bg: bool, // 是否在背景後面
}

// 精靈渲染器
#[derive(Debug)]
pub struct SpriteRenderer {
    mmu: Rc<RefCell<MMU>>,
    logger: RefCell<Logger>,
}

impl SpriteRenderer {
    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        SpriteRenderer {
            mmu,
            logger: RefCell::new(Logger::default()),
        }
    }

    /// 獲取指定掃描線上可見的精靈數量
    pub fn get_visible_sprite_count(&self, line: u8) -> Result<usize> {
        let lcdc = self.mmu.borrow().read_byte(LCD_CONTROL)?;
        if lcdc & LCDC_SPRITE_ENABLE == 0 {
            return Ok(0);
        }

        let sprite_height = if (lcdc & LCDC_SPRITE_SIZE) != 0 {
            16
        } else {
            8
        };
        let mut count = 0;

        for sprite_index in 0..40 {
            let sprite_base = 0xFE00 + sprite_index * 4;
            let y = self.mmu.borrow().read_byte(sprite_base)? as i16 - 16;

            if y <= line as i16 && (y + sprite_height as i16) > line as i16 {
                count += 1;
            }
        }

        Ok(count)
    }

    /// 渲染一條掃描線上的所有精靈
    pub fn render_line(
        &mut self,
        line: u8,
        obp0: u8,
        obp1: u8,
        display: &mut Display,
    ) -> Result<()> {
        let lcdc = self.mmu.borrow().read_byte(LCD_CONTROL)?;
        if lcdc & LCDC_SPRITE_ENABLE == 0 {
            return Ok(());
        }

        let sprite_height = if (lcdc & LCDC_SPRITE_SIZE) != 0 {
            16
        } else {
            8
        };

        // 掃描 OAM 表尋找可見的精靈
        let mut visible_sprites = Vec::new();
        for sprite_index in 0..40 {
            let sprite_base = 0xFE00 + sprite_index * 4;
            let oam_data = [
                self.mmu.borrow().read_byte(sprite_base)?,
                self.mmu.borrow().read_byte(sprite_base + 1)?,
                self.mmu.borrow().read_byte(sprite_base + 2)?,
                self.mmu.borrow().read_byte(sprite_base + 3)?,
            ];

            let sprite = Sprite::new(&oam_data);

            // 檢查精靈是否在當前掃描線上
            if sprite.y <= line as i16 && (sprite.y + sprite_height as i16) > line as i16 {
                visible_sprites.push((sprite_index, sprite));
            }
        }

        // 根據 X 座標排序精靈，X 座標相同時依據 OAM 索引（較小的優先）
        visible_sprites.sort_by(|a, b| {
            if a.1.x == b.1.x {
                a.0.cmp(&b.0)
            } else {
                a.1.x.cmp(&b.1.x)
            }
        });

        // 限制每行最多 10 個精靈
        for (_, sprite) in visible_sprites.iter().take(10) {
            let palette = if sprite.palette() { obp1 } else { obp0 };
            let flip_x = (sprite.flags & 0x20) != 0;
            let flip_y = (sprite.flags & 0x40) != 0;
            let behind_bg = (sprite.flags & 0x80) != 0;

            let mut tile_y = (line as i16 - sprite.y) as u8;
            if flip_y {
                tile_y = (sprite_height as i16 - 1 - tile_y as i16) as u8;
            }

            let tile_addr = 0x8000 + (sprite.tile_id as u16 * 16) + (tile_y as u16 * 2);
            let tile_low = self.mmu.borrow().read_byte(tile_addr)?;
            let tile_high = self.mmu.borrow().read_byte(tile_addr + 1)?;

            for x in 0..8 {
                let screen_x = sprite.x + x;
                if screen_x < 0 || screen_x >= 160 {
                    continue;
                }

                let bit = if flip_x { x } else { 7 - x };
                let color_id = ((tile_low >> bit) & 1) | (((tile_high >> bit) & 1) << 1);

                if color_id == 0 {
                    continue; // 透明像素
                }

                let color = (palette >> (color_id * 2)) & 0x03;
                let pixel_index = (line as usize * 160 + screen_x as usize) * 4;

                let rgb = match color {
                    0 => [255, 255, 255, 255], // 白色
                    1 => [192, 192, 192, 255], // 淺灰色
                    2 => [96, 96, 96, 255],    // 深灰色
                    3 => [0, 0, 0, 255],       // 黑色
                    _ => unreachable!(),
                };

                if !behind_bg || {
                    let current_color = display.buffer[pixel_index + 3];
                    current_color == 255 // 背景是白色
                } {
                    display.buffer[pixel_index..pixel_index + 4].copy_from_slice(&rgb);
                }
            }
        }

        Ok(())
    }

    pub fn render_scan_line(
        &self,
        line: u8,
        pixels: &mut Vec<Pixel>,
        obp0: u8,
        obp1: u8,
        tall_sprites: bool,
    ) -> Result<()> {
        let mmu = self.mmu.borrow();
        let lcdc = mmu.read_byte(LCD_CONTROL)?;

        // 如果精靈被禁用，直接返回
        if lcdc & LCDC_SPRITE_ENABLE == 0 {
            return Ok(());
        }

        let sprite_height = if tall_sprites { 16 } else { 8 };

        // 尋找當前掃描線上的精靈
        let mut visible_sprites = Vec::new();
        for sprite_index in 0..40 {
            let oam_addr = 0xFE00 + sprite_index * 4;
            let sprite = Sprite::new(&[
                mmu.read_byte(oam_addr)?,
                mmu.read_byte(oam_addr + 1)?,
                mmu.read_byte(oam_addr + 2)?,
                mmu.read_byte(oam_addr + 3)?,
            ]);

            if sprite.is_visible()
                && line >= sprite.y as u8
                && line < (sprite.y + sprite_height as i16) as u8
            {
                visible_sprites.push(sprite);
            }

            if visible_sprites.len() >= 10 {
                break;
            }
        }

        // 按 X 座標排序
        visible_sprites.sort_by_key(|sprite| -sprite.x);

        // 渲染每個精靈
        for sprite in visible_sprites {
            let tile_y = if sprite.flags & 0x40 != 0 {
                sprite_height - 1 - ((line as i16 - sprite.y) as u8)
            } else {
                (line as i16 - sprite.y) as u8
            };

            let tile_index = if tall_sprites {
                (sprite.tile_id & 0xFE) + (tile_y >= 8) as u8
            } else {
                sprite.tile_id
            };

            let tile_addr = 0x8000 + (tile_index as u16 * 16) + ((tile_y % 8) as u16 * 2);
            let tile_low = mmu.read_byte(tile_addr)?;
            let tile_high = mmu.read_byte(tile_addr + 1)?;

            let palette = if sprite.flags & 0x10 != 0 { obp1 } else { obp0 };

            for pixel_x in 0..8 {
                let screen_x = sprite.x
                    + if sprite.flags & 0x20 != 0 {
                        7 - pixel_x
                    } else {
                        pixel_x
                    };
                if screen_x < 0 || screen_x >= 160 {
                    continue;
                }

                let bit = if sprite.flags & 0x20 != 0 {
                    pixel_x
                } else {
                    7 - pixel_x
                };
                let color_num = ((tile_high >> bit) & 1) << 1 | ((tile_low >> bit) & 1);

                // 顏色 0 是透明的
                if color_num == 0 {
                    continue;
                }

                let attributes = sprite.flags;
                if (attributes & 0x80) != 0 && pixels[screen_x as usize][3] != 0 {
                    continue;
                }

                let palette_color = (palette >> (color_num * 2)) & 0x03;
                pixels[screen_x as usize] = map_palette_color_to_rgba(palette_color);
            }
        }

        Ok(())
    }
}
