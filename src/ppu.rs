pub struct PPU {
    pub vram: [u8; 0x2000], // 8KB VRAM
    pub oam: [u8; 0xA0],    // 160 bytes OAM
    pub lcdc: u8,           // LCD Control Register (0xFF40)
    pub stat: u8,           // LCDC Status Register (0xFF41)
    pub scy: u8,            // Scroll Y (0xFF42)
    pub scx: u8,            // Scroll X (0xFF43)
    pub ly: u8,             // LY (0xFF44)
    pub lyc: u8,            // LYC (0xFF45)
    pub bgp: u8,            // BG Palette (0xFF47)
    pub obp0: u8,           // OBJ Palette 0 (0xFF48)
    pub obp1: u8,           // OBJ Palette 1 (0xFF49)
    pub wy: u8,             // Window Y (0xFF4A)
    pub wx: u8,
    pub framebuffer: [u32; 160 * 144],             // Window X (0xFF4B)
}

impl Default for PPU {
    fn default() -> Self {
        PPU {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC,    // 預設 Game Boy palette
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            framebuffer: [0; 160 * 144],
        }
    }
}

impl PPU {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self) {
        if self.lcdc & 0x80 == 0 {
            // LCD 沒開啟，畫面全白或全黑
            return;
        }

        if self.ly < 144 {
            for px in 0..160 {
                let py: usize = self.ly as usize;
                let px: usize = px as usize;

                let use_window = (self.lcdc & 0x20 != 0)
                    && (self.ly as u8 >= self.wy)
                    && (px as u8 + 7 >= self.wx);

                let (tile_map_base, win_x, win_y) = if use_window {
                    // Window tile map
                    let base = if self.lcdc & 0x40 != 0 { 0x1C00 } else { 0x1800 };
                    let win_x = px as isize - (self.wx as isize - 7);
                    let win_y = self.ly as isize - self.wy as isize;
                    (base, win_x, win_y)
                } else {
                    // BG tile map
                    let base = if self.lcdc & 0x08 != 0 { 0x1C00 } else { 0x1800 };
                    let win_x = px.wrapping_add(self.scx as usize) as isize;
                    let win_y = py.wrapping_add(self.scy as usize) as isize;
                    (base, win_x, win_y)
                };

                if win_x < 0 || win_x >= 160 || win_y < 0 || win_y >= 144 {
                    continue;
                }

                let tile_x = (win_x as usize) / 8;
                let tile_y = (win_y as usize) / 8;
                let bg_map = &self.vram[tile_map_base..tile_map_base + 0x400];
                let tile_index = bg_map.get(tile_y * 32 + tile_x).copied().unwrap_or(0);
                let tile_data_base = if self.lcdc & 0x10 != 0 { 0x0000 } else { 0x0800 };
                let tile_data = &self.vram[tile_data_base..tile_data_base + 0x1800];
                let tile_num = if self.lcdc & 0x10 != 0 {
                    tile_index as usize
                } else {
                    ((tile_index as i8 as i16) + 128) as usize
                };
                // 背景渲染...
                let tile_addr = tile_num * 16;
                let tile_line = (win_y as usize) % 8;
                if tile_addr + tile_line * 2 + 1 >= tile_data.len() { continue; }
                let low = tile_data[tile_addr + tile_line * 2];
                let high = tile_data[tile_addr + tile_line * 2 + 1];
                let bit = 7 - ((win_x as usize) % 8);
                let color = ((high >> bit) & 1) << 1 | ((low >> bit) & 1);
                let shade = (self.bgp >> (color * 2)) & 0x03;
                let rgb = match shade {
                    0 => 0xFFFFFF, // 白
                    1 => 0xAAAAAA, // 淺灰
                    2 => 0x555555, // 深灰
                    3 => 0x000000, // 黑
                    _ => 0xFF00FF,
                };
                self.framebuffer[py * 160 + px] = rgb;

                if py == 72 && px == 80 {
                    // println!("tile_index={} shade={} bgp={:02X}", tile_index, shade, self.bgp);
                }
            }
        }
        // 推進 LY
        self.ly = self.ly.wrapping_add(1);
        if self.ly == self.lyc {
            self.stat |= 0x04; // 設 LYC=LY flag
            // 如果 STAT 中斷啟用，觸發 LCD STAT 中斷
            if self.stat & 0x40 != 0 {
                // self.mmu.request_interrupt(1); // 1 = LCD STAT
            }
        } else {
            self.stat &= !0x04;
        }

        if self.lcdc & 0x02 != 0 {
            let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
            for i in (0..160).step_by(4) {
                let y = self.oam[i] as i16 - 16;
                let x = self.oam[i + 1] as i16 - 8;
                let tile = self.oam[i + 2];
                let attr = self.oam[i + 3];
                let palette = if attr & 0x10 != 0 { self.obp1 } else { self.obp0 };
                let flip_x = attr & 0x20 != 0;
                let flip_y = attr & 0x40 != 0;
                // let priority = attr & 0x80 != 0; // 可進階處理

                // 檢查 sprite 是否在當前掃描線
                if (self.ly as i16) < y || (self.ly as i16) >= y + sprite_height {
                    continue;
                }
                let line = if !flip_y {
                    self.ly as i16 - y
                } else {
                    sprite_height - 1 - (self.ly as i16 - y)
                };
                let tile_addr = (tile as usize) * 16 + (line as usize) * 2;
                if tile_addr + 1 >= self.vram.len() {
                    continue;
                }
                let low = self.vram[tile_addr];
                let high = self.vram[tile_addr + 1];
                for px in 0..8 {
                    let sx = if !flip_x { px } else { 7 - px };
                    let bit = 7 - sx;
                    let color = ((high >> bit) & 1) << 1 | ((low >> bit) & 1);
                    if color == 0 {
                        continue; // 透明
                    }
                    let screen_x = x + px as i16;
                    if screen_x < 0 || screen_x >= 160 {
                        continue;
                    }
                    let shade = (palette >> (color * 2)) & 0x03;
                    let rgb = match shade {
                        0 => 0xFFFFFF,
                        1 => 0xAAAAAA,
                        2 => 0x555555,
                        3 => 0x000000,
                        _ => 0xFF00FF,
                    };
                    self.framebuffer[self.ly as usize * 160 + screen_x as usize] = rgb;
                }
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }
}