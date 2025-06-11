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
}

impl PPU {
    pub fn new() -> Self {
        let ppu = Self {
            vram: [0; 0x2000],
            framebuffer: vec![0xFFFFFFFFu32; 160 * 144],
            bgp: 0xE4,  // 使用標準 Game Boy 調色板 (11 10 01 00)
            obp0: 0xFF, // sprite palette 0 預設
            obp1: 0xFF, // sprite palette 1 預設
            scx: 0,
            scy: 0,
            wx: 0, // Game Boy 实际画面左上角（與MMU初始化值匹配）
            wy: 0,
            oam: [0; 160],
            lcdc: 0x91, // LCD 控制寄存器初始值 (LCD & BG 開啟，瓦片數據從 $8000-$8FFF)
            last_frame_time: std::time::Instant::now(),
            fps_counter: 0,
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
    }    pub fn step(&mut self, _mmu: &mut crate::mmu::MMU) {
        // 更新 FPS 計數器
        self.fps_counter += 1;

        // 每秒計算一次 FPS
        let now = std::time::Instant::now();
        if now.duration_since(self.last_frame_time).as_millis() > 1000 {
            self.last_frame_time = now;
            // FPS 計數器會在需要時由 get_fps 方法讀取
        }

        // 移除每幀都觸發 VBlank 中斷的錯誤邏輯
        // VBlank 中斷應該由主循環中的掃描線邏輯來觸發

        // 檢查 LCD 是否啟用 (LCDC 第 7 位)
        if (self.lcdc & 0x80) == 0 {
            // LCD 關閉，顯示更明顯的灰色背景，讓使用者知道LCD已關閉
            // 使用 memset 類似的批量操作以提高性能
            self.framebuffer.fill(0xFF666666u32); // 稍深的灰色，表示 LCD 關閉
            return;
        }

        // 檢查背景是否啟用 (LCDC 第 0 位)
        let bg_enable = (self.lcdc & 0x01) != 0; // 初始化畫面 - 使用更高效的批量操作
        self.framebuffer.fill(0xFFFFFFFFu32); // 白色背景

        // 背景和 Window 渲染
        for y in 0..144 {
            for x in 0..160 {
                let mut color = 0xFFFFFFFFu32; // 默認白色

                // 檢查Window是否啟用並且在範圍內 (LCDC 第 5 位)
                let window_enable = (self.lcdc & 0x20) != 0;
                let in_window = window_enable && y as u8 >= self.wy && x as u8 + 7 >= self.wx;

                if in_window {
                    // Window Layer
                    // 根據 LCDC 第 6 位選擇窗口瓦片地圖
                    // 0 = $9800-$9BFF, 1 = $9C00-$9FFF
                    let win_tile_map_base = if (self.lcdc & 0x40) != 0 {
                        0x1C00
                    } else {
                        0x1800
                    };

                    let wx = self.wx.saturating_sub(7);
                    let win_x = (x as i16 - wx as i16).max(0) as usize;
                    let win_y = (y as i16 - self.wy as i16).max(0) as usize;
                    let tile_x = win_x / 8;
                    let tile_y = win_y / 8;
                    if tile_x < 32 && tile_y < 32 {
                        let tile_map_addr = win_tile_map_base + tile_y * 32 + tile_x;
                        if tile_map_addr < self.vram.len() {
                            let tile_id = self.vram[tile_map_addr];
                            let pixel_x = win_x % 8;
                            let pixel_y = win_y % 8;
                            color = self.get_tile_pixel_color(tile_id, pixel_x, pixel_y, self.bgp);
                        }
                    }
                } else if bg_enable {
                    // 背景層
                    // 根據 LCDC 第 3 位選擇背景瓦片地圖
                    // 0 = $9800-$9BFF, 1 = $9C00-$9FFF
                    let bg_tile_map_base = if (self.lcdc & 0x08) != 0 {
                        0x1C00
                    } else {
                        0x1800
                    };

                    let scrolled_x = (x as u8).wrapping_add(self.scx) as usize;
                    let scrolled_y = (y as u8).wrapping_add(self.scy) as usize;
                    let tile_x = (scrolled_x / 8) % 32;
                    let tile_y = (scrolled_y / 8) % 32;
                    let tile_map_addr = bg_tile_map_base + tile_y * 32 + tile_x;

                    // 安全地取得瓦片 ID
                    if tile_map_addr < self.vram.len() {
                        let tile_id = self.vram[tile_map_addr];
                        let pixel_x = scrolled_x % 8;
                        let pixel_y = scrolled_y % 8;
                        color = self.get_tile_pixel_color(tile_id, pixel_x, pixel_y, self.bgp);
                    }
                } // 由於我們確保 x < 160 且 y < 144，這裡可以直接寫入而無需檢查
                let fb_idx = y * 160 + x;
                self.framebuffer[fb_idx] = color;
            }
        }

        // 檢查 Sprite (物體) 是否啟用 (LCDC 第 1 位)
        let sprite_enable = (self.lcdc & 0x02) != 0;

        if sprite_enable {
            // 檢查 Sprite 大小 (LCDC 第 2 位)
            // 0 = 8x8, 1 = 8x16
            let sprite_size = if (self.lcdc & 0x04) != 0 { 16 } else { 8 };

            // Sprite 渲染（OAM 疊加）
            for i in 0..40 {
                let base = i * 4;
                let y_pos = self.oam[base] as i16 - 16;
                let x_pos = self.oam[base + 1] as i16 - 8;
                let tile_idx = self.oam[base + 2] as usize;
                let attr = self.oam[base + 3];
                let flip_x = (attr & 0x20) != 0;
                let flip_y = (attr & 0x40) != 0;
                let priority = (attr & 0x80) != 0;

                // 根據精靈屬性選擇使用 OBP0 或 OBP1 調色板
                let palette_number = (attr & 0x10) != 0;
                let palette = if palette_number { self.obp1 } else { self.obp0 };

                // 如果座標超出畫面，跳過這個精靈
                if x_pos <= -8 || x_pos >= 160 || y_pos <= -16 || y_pos >= 144 {
                    continue;
                }

                // 如果使用 8x16 模式，低位需要調整
                let mut tiles_to_render: [usize; 2] = [0, 0];
                let tiles_len = if sprite_size == 16 {
                    // 8x16 模式中，忽略最低位
                    tiles_to_render[0] = tile_idx & 0xFE;
                    tiles_to_render[1] = (tile_idx & 0xFE) + 1;
                    2
                } else {
                    // 8x8 模式
                    tiles_to_render[0] = tile_idx;
                    1
                };

                // 渲染每個精靈瓦片
                for tile_offset in 0..tiles_len {
                    let tile = tiles_to_render[tile_offset];
                    for py in 0..8 {
                        let real_py = if sprite_size == 16 {
                            py + tile_offset * 8
                        } else {
                            py
                        };
                        let sy = if flip_y {
                            sprite_size - 1 - real_py
                        } else {
                            real_py
                        };
                        let screen_y = y_pos + sy as i16;

                        if screen_y < 0 || screen_y >= 144 {
                            continue;
                        }

                        for px in 0..8 {
                            let sx = if flip_x { 7 - px } else { px };
                            let screen_x = x_pos + px;

                            if screen_x < 0 || screen_x >= 160 {
                                continue;
                            } // 獲取精靈瓦片數據
                            let tile_addr = tile * 16 + (sy as usize % 8) * 2;

                            if tile_addr + 1 >= self.vram.len() {
                                continue;
                            } // 使用優化的瓦片數據讀取，減少額外函數調用
                            let low = if tile_addr < self.vram.len() {
                                self.vram[tile_addr]
                            } else {
                                0
                            };
                            let high = if tile_addr + 1 < self.vram.len() {
                                self.vram[tile_addr + 1]
                            } else {
                                0
                            };
                            let bit = 7 - sx as u8;
                            let lo = (low >> bit) & 1;
                            let hi = (high >> bit) & 1;
                            let color_id = (hi << 1) | lo;

                            // 顏色 0 是透明的
                            if color_id == 0 {
                                continue;
                            } // 使用輔助函數獲取精靈顏色
                            let color = self.get_color_from_palette(palette, color_id); // 座標合法檢查已經在外部進行，這裡可以直接訪問
                            let idx = (screen_y as usize) * 160 + (screen_x as usize);

                            // 精靈優先級處理：
                            // 1. priority=0 時精靈總是在前景
                            // 2. priority=1 時精靈在背景有顏色的區域後面
                            let current_bg = self.framebuffer[idx];
                            if !priority || current_bg == 0xFFFFFFFFu32 {
                                self.framebuffer[idx] = color;
                            }
                        }
                    }
                }
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
                self.scx, self.scy,
                self.wx, self.wy,
                self.vram.iter().filter(|&&b| b != 0).count(),
                self.oam.chunks(4).filter(|sprite| sprite[0] != 0 || sprite[1] != 0).count()
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
}
