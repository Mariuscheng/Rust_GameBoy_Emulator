use crate::mmu::MMU;

pub struct PPU {
    pub vram: [u8; 0x2000],                  // 8KB VRAM
    framebuffer: Vec<u32>,                   // 160x144 畫面
    pub bgp: u8,                             // 背景調色板
    pub obp0: u8,                            // sprite palette 0
    pub obp1: u8,                            // sprite palette 1
    pub scx: u8,                             // 背景水平滾动
    pub scy: u8,                             // 背景垂直滾动
    pub wx: u8,                              // Window X
    pub wy: u8,                              // Window Y
    pub oam: [u8; 160],                      // 40 sprites * 4 bytes
    pub lcdc: u8,                            // LCD 控制寄存器
    pub last_frame_time: std::time::Instant, // 上一幀的時間
    pub fps_counter: u32,                    // FPS 計數器
    pub mode: u8,                            // PPU 模式 (0-3)
    pub ly: u8,                              // 當前掃描線 (LY)
    pub lyc: u8,                             // LY 比較值 (LYC)
    pub stat: u8,                            // STAT 寄存器
    pub dots: u32,                           // 點時鐘計數器
}

impl PPU {
    pub fn new() -> Self {
        let ppu = Self {
            vram: [0; 0x2000],
            framebuffer: vec![0xFFFFFFFFu32; 160 * 144],
            bgp: 0xE4, // 使用標準 Game Boy 調色板
            obp0: 0xFF,
            obp1: 0xFF,
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            oam: [0; 160],
            lcdc: 0x91,
            last_frame_time: std::time::Instant::now(),
            fps_counter: 0,
            mode: 0,
            ly: 0,
            lyc: 0,
            stat: 0,
            dots: 0,
        };
        ppu
    }
    pub fn set_bgp(&mut self, value: u8) {
        self.bgp = value;
    }
    pub fn set_obp0(&mut self, value: u8) {
        self.obp0 = value;
    }
    pub fn set_obp1(&mut self, value: u8) {
        self.obp1 = value;
    }
    pub fn set_scx(&mut self, value: u8) {
        self.scx = value;
    }
    pub fn set_scy(&mut self, value: u8) {
        self.scy = value;
    }
    pub fn set_wx(&mut self, value: u8) {
        self.wx = value;
    }
    pub fn set_wy(&mut self, value: u8) {
        self.wy = value;
    }
    pub fn set_lcdc(&mut self, value: u8) {
        self.lcdc = value;
    }
    pub fn set_oam(&mut self, data: [u8; 160]) {
        self.oam = data;
    }
    pub fn step(&mut self, mmu: &mut crate::mmu::MMU) {
        // 更新掃描線時序
        self.update_mode(mmu);

        // 如果LCD關閉，清空畫面並返回
        if (self.lcdc & 0x80) == 0 {
            self.framebuffer.fill(0xFF666666u32);
            return;
        }

        // 根據當前模式執行相應操作
        match self.mode {
            0 => { // H-Blank
                // 在H-Blank期間不需要執行渲染操作
            }
            1 => { // V-Blank
                // 在V-Blank期間不需要執行渲染操作
            }
            2 => {
                // OAM Scan
                self.scan_oam();
            }
            3 => {
                // Drawing
                if self.ly < 144 {
                    self.render_scanline();
                }
            }
            _ => unreachable!(),
        }
    }

    fn update_mode(&mut self, mmu: &mut crate::mmu::MMU) {
        // 更新點時鐘
        self.dots += 1;

        // 一條掃描線總共需要456個點時鐘
        if self.dots >= 456 {
            self.dots = 0;
            self.ly = (self.ly + 1) % 154;

            // 檢查 LY=LYC 中斷
            if self.ly == self.lyc {
                self.stat |= 0x04; // 設置巧合標誌
                if (self.stat & 0x40) != 0 {
                    mmu.if_reg |= 0x02; // 觸發STAT中斷
                }
            } else {
                self.stat &= !0x04; // 清除巧合標誌
            }
        }

        let old_mode = self.mode;

        // 更新PPU模式
        self.mode = if self.ly >= 144 {
            1 // V-Blank
        } else {
            if self.dots <= 80 {
                2 // OAM Scan
            } else if self.dots <= 252 {
                3 // Drawing
            } else {
                0 // H-Blank
            }
        };

        // 更新STAT寄存器的模式位
        self.stat = (self.stat & 0xFC) | self.mode;

        // 檢查模式變更中斷
        if old_mode != self.mode {
            match self.mode {
                0 => {
                    // H-Blank
                    if (self.stat & 0x08) != 0 {
                        mmu.if_reg |= 0x02;
                    }
                }
                1 => {
                    // V-Blank
                    mmu.if_reg |= 0x01; // 觸發V-Blank中斷
                    if (self.stat & 0x10) != 0 {
                        mmu.if_reg |= 0x02;
                    }
                }
                2 => {
                    // OAM
                    if (self.stat & 0x20) != 0 {
                        mmu.if_reg |= 0x02;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    // 獲取瓦片像素顏色的輔助方法
    fn get_tile_pixel_color(
        &self,
        tile_id: u8,
        pixel_x: usize,
        pixel_y: usize,
        palette: u8,
    ) -> u32 {
        // 根據 LCDC 第 4 位選擇不同的瓦片數據區域
        // 0 = 0x8800-0x97FF，使用有符號編號（-128到127）
        // 1 = 0x8000-0x8FFF，使用無符號編號（0到255）
        let tile_data_addr;
        if (self.lcdc & 0x10) != 0 {
            // 使用 0x8000-0x8FFF (VRAM 0x0000-0x0FFF)
            tile_data_addr = (tile_id as usize) * 16 + pixel_y * 2;
        } else {
            // 使用 0x8800-0x97FF，將 tile_id 視為有符號整數
            let signed_id = tile_id as i8;
            // 0x9000 實際上是 0x1000 在 VRAM 陣列中
            tile_data_addr = 0x1000 + ((signed_id as i16) + 128) as usize * 16 + pixel_y * 2;
        } // 確保地址在有效範圍內
        if tile_data_addr + 1 >= self.vram.len() {
            println!(
                "Warning: Tile address out of bounds: {:04X}",
                tile_data_addr
            );
            return 0xFFFFFFFFu32; // 如果超出範圍，返回白色
        }

        let low_byte = self.safe_vram_read(tile_data_addr);
        let high_byte = self.safe_vram_read(tile_data_addr + 1);

        let bit_pos = 7 - pixel_x;
        let low_bit = (low_byte >> bit_pos) & 1;
        let high_bit = (high_byte >> bit_pos) & 1;
        let color_id = (high_bit << 1) | low_bit; // 使用輔助函數從調色板獲取實際顏色
        self.get_color_from_palette(palette, color_id)
    } // 根據給定的調色板和顏色ID獲取RGB顏色
    fn get_color_from_palette(&self, palette: u8, color_id: u8) -> u32 {
        // 優化：直接使用位運算並避免 match 語句的開銷
        let shade = (palette >> (color_id * 2)) & 0b11;
        // 使用更準確的 Game Boy 顏色（稍微調整灰度以更接近原始體驗）
        const COLORS: [u32; 4] = [
            0xFFFFFFFFu32, // 白色 (最亮)
            0xFFB0B0B0u32, // 淺灰 (稍微調暗)
            0xFF686868u32, // 深灰 (稍微調亮)
            0xFF000000u32, // 黑色 (最暗)
        ];

        // 安全訪問陣列，理論上 shade 應該總是 0-3 內，但為了避免可能的非法位模式
        COLORS[shade as usize & 0x3]
    }
    pub fn debug_info(&self, frame_count: u64) -> String {
        // 每 200 幀輸出一次詳細調試資訊
        if frame_count % 200 == 0 {
            // 解析 LCDC 各個位元的含義
            let lcdc_details = format!(
                "LCD顯示開啟: {}, 視窗區域: {}, 視窗啟用: {}, 瓦片數據區域: {}, \
                BG瓦片地圖: {}, Sprite大小: {}, Sprite啟用: {}, BG顯示: {}",
                (self.lcdc & 0x80) != 0,
                if (self.lcdc & 0x40) != 0 {
                    "0x9C00-0x9FFF"
                } else {
                    "0x9800-0x9BFF"
                },
                (self.lcdc & 0x20) != 0,
                if (self.lcdc & 0x10) != 0 {
                    "0x8000-0x8FFF"
                } else {
                    "0x8800-0x97FF"
                },
                if (self.lcdc & 0x08) != 0 {
                    "0x9C00-0x9FFF"
                } else {
                    "0x9800-0x9BFF"
                },
                if (self.lcdc & 0x04) != 0 {
                    "8x16"
                } else {
                    "8x8"
                },
                (self.lcdc & 0x02) != 0,
                (self.lcdc & 0x01) != 0
            );

            // 輸出更完整的調色板信息
            format!(
                "PPU DEBUG (幀 {}):\n  LCDC: 0x{:02X} [{}]\n  調色板: BGP=0x{:02X}, OBP0=0x{:02X}, OBP1=0x{:02X}\n  \
                滾動: SCX/SCY={}/{}, WX/WY={}/{}\n  VRAM非零位元組: {}\n  \
                OAM使用: {} sprites",
                frame_count,
                self.lcdc,
                lcdc_details,
                self.bgp,
                self.obp0,
                self.obp1,
                self.scx,
                self.scy,
                self.wx,
                self.wy,
                self.vram.iter().filter(|&&b| b != 0).count(),
                self.oam
                    .chunks(4)
                    .filter(|sprite| sprite[0] != 0 || sprite[1] != 0)
                    .count()
            )
        } else {
            String::new()
        }
    }

    pub fn get_fps(&mut self) -> u32 {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        self.fps_counter += 1;

        if elapsed.as_millis() > 1000 {
            let fps = (self.fps_counter as f32 / elapsed.as_secs_f32()).round() as u32;
            self.fps_counter = 0;
            self.last_frame_time = now;
            return fps;
        }

        0 // 如果不到1秒，返回0表示不更新FPS顯示
    }

    // 安全地從 VRAM 中讀取字節
    fn safe_vram_read(&self, addr: usize) -> u8 {
        if addr < self.vram.len() {
            self.vram[addr]
        } else {
            // 如果地址超出範圍，則返回 0
            0
        }
    }

    // 獲取瓦片原始數據，用於調試
    pub fn get_tile_data(&self, tile_id: u8) -> Vec<u8> {
        let mut tile_data = Vec::with_capacity(16);

        // 根據 LCDC 第 4 位選擇不同的瓦片數據區域
        let base_addr = if (self.lcdc & 0x10) != 0 {
            // 使用 0x8000-0x8FFF (VRAM 0x0000-0x0FFF)
            (tile_id as usize) * 16
        } else {
            // 使用 0x8800-0x97FF，將 tile_id 視為有符號整數
            let signed_id = tile_id as i8;
            0x1000 + ((signed_id as i16) + 128) as usize * 16
        };

        // 獲取瓦片的16個字節
        for i in 0..16 {
            if base_addr + i < self.vram.len() {
                tile_data.push(self.vram[base_addr + i]);
            } else {
                tile_data.push(0);
            }
        }

        tile_data
    }

    pub fn set_stat(&mut self, value: u8) {
        // 只允許寫入高 5 位，保留當前模式和 LYC=LY 標誌
        self.stat = (value & 0xF8) | (self.stat & 0x07);
    }

    pub fn get_stat(&self) -> u8 {
        self.stat
    }

    pub fn set_lyc(&mut self, value: u8) {
        self.lyc = value;
    }

    pub fn get_ly(&self) -> u8 {
        self.ly
    }

    pub fn get_lyc(&self) -> u8 {
        self.lyc
    }

    fn scan_oam(&mut self) {
        // OAM掃描階段，蒐集當前掃描線上可見的精靈
        let mut visible_sprites = Vec::with_capacity(10);

        for i in 0..40 {
            let base = i * 4;
            let y_pos = self.oam[base] as i16 - 16;
            let x_pos = self.oam[base + 1] as i16 - 8;
            let sprite_size = if (self.lcdc & 0x04) != 0 { 16 } else { 8 };

            // 檢查精靈是否在當前掃描線上
            if (y_pos <= self.ly as i16) && ((y_pos + sprite_size) > self.ly as i16) {
                if visible_sprites.len() < 10 {
                    visible_sprites.push(i);
                }
            }
        }
    }

    fn render_scanline(&mut self) {
        // 背景渲染
        if (self.lcdc & 0x01) != 0 {
            self.render_background();
        }

        // 窗口渲染
        if (self.lcdc & 0x20) != 0 {
            self.render_window();
        }

        // 精靈渲染
        if (self.lcdc & 0x02) != 0 {
            self.render_sprites();
        }
    }

    fn render_background(&mut self) {
        let bg_tile_map = if (self.lcdc & 0x08) != 0 {
            0x1C00
        } else {
            0x1800
        };
        let tile_data = if (self.lcdc & 0x10) != 0 {
            0x0000
        } else {
            0x1000
        };

        let y_pos = (self.ly as u16 + self.scy as u16) & 0xFF;
        let tile_y = (y_pos / 8) as usize;

        for x in 0..160 {
            let x_pos = (x as u16 + self.scx as u16) & 0xFF;
            let tile_x = (x_pos / 8) as usize;
            let tile_index = self.vram[bg_tile_map + tile_y * 32 + tile_x];

            let tile_addr = if (self.lcdc & 0x10) != 0 {
                tile_data + (tile_index as u16 * 16)
            } else {
                tile_data + ((tile_index as i8 as i16 + 128) as u16 * 16)
            };

            let py = (y_pos % 8) as usize;
            let px = (x_pos % 8) as usize;

            let byte1 = self.vram[tile_addr as usize + py * 2];
            let byte2 = self.vram[tile_addr as usize + py * 2 + 1];

            let color_bit = 7 - px;
            let color_num = ((byte2 >> color_bit) & 1) << 1 | ((byte1 >> color_bit) & 1);
            let color = match (self.bgp >> (color_num * 2)) & 0x03 {
                0 => 0xFFFFFFFF, // White
                1 => 0xFFAAAAAA, // Light gray
                2 => 0xFF555555, // Dark gray
                3 => 0xFF000000, // Black
                _ => unreachable!(),
            };

            let fb_index = (self.ly as usize * 160 + x) as usize;
            self.framebuffer[fb_index] = color;
        }
    }

    fn render_window(&mut self) {
        if self.wy > self.ly || self.wx > 166 {
            return;
        }

        let win_tile_map = if (self.lcdc & 0x40) != 0 {
            0x1C00
        } else {
            0x1800
        };
        let tile_data = if (self.lcdc & 0x10) != 0 {
            0x0000
        } else {
            0x1000
        };

        let win_y = self.ly as i16 - self.wy as i16;
        if win_y < 0 {
            return;
        }

        let tile_y = (win_y as u16 / 8) as usize;

        for x in 0..160 {
            let win_x = x as i16 - (self.wx as i16 - 7);
            if win_x < 0 {
                continue;
            }

            let tile_x = (win_x as u16 / 8) as usize;
            let tile_index = self.vram[win_tile_map + tile_y * 32 + tile_x];

            let tile_addr = if (self.lcdc & 0x10) != 0 {
                tile_data + (tile_index as u16 * 16)
            } else {
                tile_data + ((tile_index as i8 as i16 + 128) as u16 * 16)
            };

            let py = (win_y % 8) as usize;
            let px = (win_x % 8) as usize;

            let byte1 = self.vram[tile_addr as usize + py * 2];
            let byte2 = self.vram[tile_addr as usize + py * 2 + 1];

            let color_bit = 7 - px;
            let color_num = ((byte2 >> color_bit) & 1) << 1 | ((byte1 >> color_bit) & 1);
            let color = match (self.bgp >> (color_num * 2)) & 0x03 {
                0 => 0xFFFFFFFF,
                1 => 0xFFAAAAAA,
                2 => 0xFF555555,
                3 => 0xFF000000,
                _ => unreachable!(),
            };

            let fb_index = (self.ly as usize * 160 + x) as usize;
            self.framebuffer[fb_index] = color;
        }
    }

    fn render_sprites(&mut self) {
        let sprite_size = if (self.lcdc & 0x04) != 0 { 16 } else { 8 };

        for i in (0..40).rev() {
            let base = i * 4;
            let y_pos = self.oam[base] as i16 - 16;
            let x_pos = self.oam[base + 1] as i16 - 8;
            let tile_num = self.oam[base + 2];
            let attributes = self.oam[base + 3];

            if y_pos > self.ly as i16 || y_pos + sprite_size <= self.ly as i16 {
                continue;
            }

            let use_obp1 = (attributes & 0x10) != 0;
            let x_flip = (attributes & 0x20) != 0;
            let y_flip = (attributes & 0x40) != 0;
            let priority = (attributes & 0x80) != 0;

            let palette = if use_obp1 { self.obp1 } else { self.obp0 };

            let mut tile_y = self.ly as i16 - y_pos;
            if y_flip {
                tile_y = (sprite_size - 1) - tile_y;
            }

            let tile_addr = (tile_num as u16 * 16 + (tile_y as u16 * 2)) as usize;
            let byte1 = self.vram[tile_addr];
            let byte2 = self.vram[tile_addr + 1];

            for x in 0..8 {
                let screen_x = x_pos + x;
                if screen_x < 0 || screen_x >= 160 {
                    continue;
                }

                let bit = if x_flip { x } else { 7 - x };
                let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);

                if color_num == 0 {
                    continue; // 透明色
                }

                let color = match (palette >> (color_num * 2)) & 0x03 {
                    0 => 0xFFFFFFFF,
                    1 => 0xFFAAAAAA,
                    2 => 0xFF555555,
                    3 => 0xFF000000,
                    _ => unreachable!(),
                };

                let fb_index = (self.ly as usize * 160 + screen_x as usize) as usize;

                if !priority || self.framebuffer[fb_index] == 0xFFFFFFFF {
                    self.framebuffer[fb_index] = color;
                }
            }
        }
    }
}
