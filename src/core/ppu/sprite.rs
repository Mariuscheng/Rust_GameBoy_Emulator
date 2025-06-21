//! 精靈渲染器，產生精靈圖層像素

use crate::core::mmu::MMU;
use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub struct SpriteFlags(u8);

impl SpriteFlags {
    pub fn new(value: u8) -> Self {
        SpriteFlags(value)
    }

    pub fn priority(&self) -> bool {
        (self.0 & 0x80) != 0
    }

    pub fn y_flip(&self) -> bool {
        (self.0 & 0x40) != 0
    }

    pub fn x_flip(&self) -> bool {
        (self.0 & 0x20) != 0
    }

    pub fn palette(&self) -> bool {
        (self.0 & 0x10) != 0
    }
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub y: u8,
    pub x: u8,
    pub tile: u8,
    pub flags: SpriteFlags,
}

impl Sprite {
    pub fn new(y: u8, x: u8, tile: u8, flags: u8) -> Self {
        Self {
            y,
            x,
            tile,
            flags: SpriteFlags::new(flags),
        }
    }
}

#[derive(Debug, Default)]
pub struct SpriteRenderer {
    sprites: Vec<Sprite>,
    sprite_height: u8,
}

impl SpriteRenderer {
    pub fn new() -> Self {
        Self {
            sprites: Vec::with_capacity(40),
            sprite_height: 8,
        }
    }

    pub fn update_sprites(&mut self, mmu: &MMU) -> Result<()> {
        self.sprites.clear();
        
        // 從 OAM 中讀取精靈資料
        for i in 0..40 {
            let base = 0xFE00 + (i * 4);
            let y = mmu.read_byte(base as u16)?;
            let x = mmu.read_byte((base + 1) as u16)?;
            let tile = mmu.read_byte((base + 2) as u16)?;
            let flags = mmu.read_byte((base + 3) as u16)?;
            
            self.sprites.push(Sprite::new(y, x, tile, flags));
        }
        
        // 根據 X 座標排序精靈，優先度高的放在後面
        self.sprites.sort_by(|a, b| a.x.cmp(&b.x));
        Ok(())
    }

    pub fn render_line(&self, line: u8, mmu: &MMU) -> Result<Vec<Option<u8>>> {
        let mut line_buffer = vec![None; 160];
        
        for sprite in self.sprites.iter().rev() {
            let y_pos = sprite.y.wrapping_sub(16);
            
            // 檢查精靈是否在當前掃描線上
            if line < y_pos || line >= y_pos.wrapping_add(self.sprite_height) {
                continue;
            }
            
            let y_offset = line.wrapping_sub(y_pos);
            let y = if sprite.flags.y_flip() { 
                self.sprite_height.wrapping_sub(1).wrapping_sub(y_offset)
            } else { 
                y_offset 
            };
            
            // 取得圖塊資料
            let tile_addr = 0x8000 + (sprite.tile as u16 * 16) + ((y as u16) * 2);
            let tile_low = mmu.read_byte(tile_addr)?;
            let tile_high = mmu.read_byte(tile_addr + 1)?;
            
            let x_pos = sprite.x.wrapping_sub(8);
            
            // 遍歷精靈的每個像素
            for bit_pos in 0..8u8 {
                let screen_x = x_pos.wrapping_add(if sprite.flags.x_flip() {
                    7 - bit_pos
                } else {
                    bit_pos
                });
                
                // 檢查是否在螢幕範圍內
                if screen_x >= 160 {
                    continue;
                }
                
                let color_low = ((tile_low >> bit_pos) & 0x1) as u8;
                let color_high = ((tile_high >> bit_pos) & 0x1) as u8;
                let color_num = (color_high << 1) | color_low;
                
                // 顏色 0 是透明
                if color_num == 0 {
                    continue;
                }
                
                // 設置顏色
                line_buffer[screen_x as usize] = Some(color_num);
            }
        }
        
        Ok(line_buffer)
    }
    
    pub fn set_sprite_height(&mut self, height: u8) {
        self.sprite_height = height;
    }
}
