use super::registers::{SCREEN_WIDTH, apply_palette};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Sprite {
    y: u8,          // Y 位置 (實際位置 = y - 16)
    x: u8,          // X 位置 (實際位置 = x - 8)
    tile_index: u8, // 圖塊索引
    attributes: u8, // 精靈屬性
}

#[allow(dead_code)]
impl Sprite {
    pub fn new(oam_data: &[u8], index: usize) -> Self {
        let base = index * 4;
        Self {
            y: oam_data[base],
            x: oam_data[base + 1],
            tile_index: oam_data[base + 2],
            attributes: oam_data[base + 3],
        }
    }

    pub fn is_visible_on_line(&self, line: u8, sprite_size: u8) -> bool {
        let sprite_height = if sprite_size != 0 { 16 } else { 8 };
        let y = line.wrapping_add(16);
        y >= self.y && y < self.y.wrapping_add(sprite_height)
    }

    pub fn get_priority(&self) -> bool {
        self.attributes & 0x80 != 0
    }

    pub fn is_y_flipped(&self) -> bool {
        self.attributes & 0x40 != 0
    }

    pub fn is_x_flipped(&self) -> bool {
        self.attributes & 0x20 != 0
    }

    pub fn get_palette(&self) -> bool {
        self.attributes & 0x10 != 0
    }
}

#[allow(dead_code)]
pub struct SpriteRenderer {
    visible_sprites: Vec<Sprite>,
}

#[allow(dead_code)]
impl SpriteRenderer {
    pub fn new() -> Self {
        Self {
            visible_sprites: Vec::with_capacity(40),
        }
    }

    pub fn scan_line(&mut self, oam: &[u8], line: u8, sprite_size: u8) {
        self.visible_sprites.clear();

        // 掃描 OAM 尋找可見的精靈
        for i in 0..40 {
            let sprite = Sprite::new(oam, i);
            if sprite.is_visible_on_line(line, sprite_size) {
                self.visible_sprites.push(sprite);
                if self.visible_sprites.len() >= 10 {
                    break; // 每條掃描線最多顯示 10 個精靈
                }
            }
        }

        // 根據 X 座標排序精靈 (X 較大的優先)
        self.visible_sprites.sort_by_key(|s| -(s.x as i16));
    }

    pub fn render_line(
        &self,
        vram: &[u8],
        line: u8,
        sprite_size: u8,
        obp0: u8,
        obp1: u8,
    ) -> [Option<u8>; SCREEN_WIDTH] {
        let mut line_buffer = [None; SCREEN_WIDTH];

        for sprite in &self.visible_sprites {
            let palette = if sprite.get_palette() { obp1 } else { obp0 };

            // 計算精靈在掃描線上的位置
            let sprite_line = line.wrapping_add(16).wrapping_sub(sprite.y);
            let flipped_y = if sprite.is_y_flipped() {
                if sprite_size != 0 {
                    15 - sprite_line
                } else {
                    7 - sprite_line
                }
            } else {
                sprite_line
            };

            // 取得圖塊數據
            let tile_index = if sprite_size != 0 {
                sprite.tile_index & 0xFE | ((sprite_line >= 8) as u8)
            } else {
                sprite.tile_index
            };

            let tile_addr = tile_index as usize * 16 + (flipped_y as usize & 7) * 2;
            let low_byte = vram[tile_addr];
            let high_byte = vram[tile_addr + 1];

            // 渲染精靈的像素
            for x in 0..8 {
                let screen_x = sprite.x.wrapping_add(x).wrapping_sub(8);
                if screen_x >= SCREEN_WIDTH as u8 {
                    continue;
                }

                let bit = if sprite.is_x_flipped() { x } else { 7 - x };
                let color = ((high_byte >> bit) & 1) << 1 | ((low_byte >> bit) & 1);

                if color != 0 {
                    // 顏色 0 是透明的
                    let final_color = apply_palette(color, palette);
                    line_buffer[screen_x as usize] = Some(final_color);
                }
            }
        }

        line_buffer
    }
}
