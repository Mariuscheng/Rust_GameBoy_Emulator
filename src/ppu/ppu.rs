use crate::utils::Logger;

/// PPU (像素處理單元)負責 Game Boy 的圖形渲染
#[derive(Debug)]
pub struct PPU {
    /// 8KB 影像記憶體,用於儲存圖塊數據和背景地圖
    pub vram: [u8; 0x2000],

    /// 160x144 畫面緩衝區
    framebuffer: Vec<u32>,

    /// FF47 - BGP - 背景調色板數據
    /// 位元 7-6: 顏色 3 (11: 黑, 10: 深灰, 01: 淺灰, 00: 白)
    /// 位元 5-4: 顏色 2
    /// 位元 3-2: 顏色 1
    /// 位元 1-0: 顏色 0
    pub bgp: u8,

    /// FF48 - OBP0 - 物件調色板 0 數據
    /// 同 BGP 格式但位元 1-0 透明
    pub obp0: u8,

    /// FF49 - OBP1 - 物件調色板 1 數據
    /// 同 BGP 格式但位元 1-0 透明
    pub obp1: u8,

    /// FF43 - SCX - 背景水平捲動位置 (0-255)
    pub scx: u8,

    /// FF42 - SCY - 背景垂直捲動位置 (0-255)
    pub scy: u8,

    /// FF4B - WX - 視窗 X 位置減 7 (0-166)
    pub wx: u8,

    /// FF4A - WY - 視窗 Y 位置 (0-143)
    pub wy: u8,

    /// FF40 - LCDC - LCD 控制寄存器
    /// 位元 7: LCD 顯示開啟
    /// 位元 6: 視窗瓦片地圖選擇
    /// 位元 5: 視窗顯示開啟
    /// 位元 4: 背景/視窗瓦片數據選擇
    /// 位元 3: 背景瓦片地圖選擇
    /// 位元 2: 物件(Sprite)大小
    /// 位元 1: 物件顯示開啟
    /// 位元 0: 背景顯示開啟
    pub lcdc: u8,

    /// 用於 FPS 計算的時間點
    pub last_frame_time: std::time::Instant,

    /// FPS 計數器
    pub fps_counter: u32,

    /// 目前 PPU 模式 (0-3)
    /// 0: H-Blank
    /// 1: V-Blank
    /// 2: OAM Scan
    /// 3: Drawing
    pub mode: u8,

    /// FF44 - LY - 目前掃描線 (0-153)
    pub ly: u8,

    /// FF45 - LYC - 掃描線比較值
    pub lyc: u8,

    /// FF41 - STAT - LCD 狀態寄存器
    /// 位元 6: LYC=LY 中斷開啟
    /// 位元 5: Mode 2 中斷開啟
    /// 位元 4: Mode 1 中斷開啟
    /// 位元 3: Mode 0 中斷開啟
    /// 位元 2: LYC=LY 標誌
    /// 位元 1-0: 目前模式
    pub stat: u8,

    /// 點時鐘計數器
    pub dots: u32,

    /// Sprite 屬性表 (40個物件 * 4位元組)
    pub oam: [u8; 160],
}

impl PPU {
    pub fn new() -> Self {
        let ppu = Self {
            vram: [0; 0x2000],
            framebuffer: vec![0xFFFFFFFFu32; 160 * 144],
            bgp: 0xFC, // 修改為更適合顯示文字的調色板 (11111100)
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

        // 初始化測試圖案到VRAM
        // ppu.initialize_test_patterns();

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
        // 在 Game Boy 上，WX 超出範圍的值會被正常設置，
        // 但窗口只有在有效範圍時才會被繪製
        // 完全保存原始值以更準確地模擬硬體行為
        self.wx = value;
    }

    pub fn set_wy(&mut self, value: u8) {
        // 在 Game Boy 上，WY 超出範圍的值會被正常設置，
        // 但窗口只有在有效範圍時才會被繪製
        // 完全保存原始值以更準確地模擬硬體行為
        self.wy = value;
    }

    pub fn set_lcdc(&mut self, value: u8) {
        self.lcdc = value;
    }
    pub fn set_oam(&mut self, data: [u8; 160]) {
        self.oam = data;
    }
    pub fn step(&mut self, cycles: u8, mmu: &mut crate::mmu::MMU) {
        // 自動 debug log
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
            let _ = writeln!(file, "[PPU] step: LY={}, LCDC=0x{:02X}, STAT=0x{:02X}, mode={}, VBlank={} ({} cycles)",
                self.ly, self.lcdc, self.stat, self.mode, if self.ly >= 144 {"Y"} else {"N"}, self.dots);
        }
        // 推進 PPU 狀態機
        let old_ly = self.ly;
        self.dots += cycles as u32;
        if self.dots >= 456 {
            self.dots -= 456;
            self.ly = self.ly.wrapping_add(1);
            if self.ly > 153 {
                self.ly = 0;
            }
            // DEBUG: 掃描線推進 log
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                let _ = writeln!(file, "[PPU-LY] ly advanced: {} -> {}", old_ly, self.ly);
            }
        }
        // 如果LCD關閉，清空畫面並返回
        if (self.lcdc & 0x80) == 0 {
            self.framebuffer.fill(0xFF666666u32);
            return;
        }
        // 根據當前模式執行相應操作
        match self.mode {
            0 => { /* H-Blank */ },
            1 => { /* V-Blank */ },
            2 => { self.scan_oam(); },
            3 => { if self.ly < 144 { self.render_scanline(); } },
            _ => unreachable!(),
        }

        // 根據 dots/LY 自動切換 mode
        if self.ly >= 144 {
            self.mode = 1; // V-Blank
        } else if self.dots < 80 {
            self.mode = 2; // OAM Scan
        } else if self.dots < 252 {
            self.mode = 3; // Drawing
        } else {
            self.mode = 0; // H-Blank
        }
        // 更新 STAT 寄存器的 mode bits (低兩位)
        self.stat = (self.stat & 0xFC) | (self.mode & 0x03);

        // 檢查模式變更中斷
        if self.mode == 0 {
            // H-Blank
            if (self.stat & 0x08) != 0 {
                mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
            }
        } else if self.mode == 1 {
            // V-Blank
            mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x01);
            if (self.stat & 0x10) != 0 {
                mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
            }
        } else if self.mode == 2 {
            // OAM Scan
            if (self.stat & 0x20) != 0 {
                mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
            }
        }
    }

    /// 記錄 PPU 狀態變更
    fn log_state_change(&self, _old_mode: u8, _new_mode: u8) {
        // log: PPU Mode Change: {} -> {} at LY={}
    }

    /// PPU 模式更新
    fn update_mode(&mut self, mmu: &mut crate::mmu::MMU) {
        let old_mode = self.mode;

        // 更新 PPU 模式
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

        // 如果模式發生變化,記錄並通知
        if old_mode != self.mode {
            self.log_state_change(old_mode, self.mode);
        }

        // 更新 STAT 寄存器的模式位
        self.stat = (self.stat & 0xFC) | self.mode;

        // 檢查模式變更中斷
        if old_mode != self.mode {
            match self.mode {
                0 => {
                    // H-Blank
                    if (self.stat & 0x08) != 0 {
                        mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
                    }
                }
                1 => {
                    // V-Blank
                    mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x01);
                    if (self.stat & 0x10) != 0 {
                        mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
                    }
                }
                2 => {
                    // OAM Scan
                    if (self.stat & 0x20) != 0 {
                        mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
                    }
                }
                _ => {}
            }
        }
    }
    /// 獲取畫面緩衝區的引用
    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    /// 獲取可變的畫面緩衝區引用
    pub fn get_framebuffer_mut(&mut self) -> &mut [u32] {
        &mut self.framebuffer
    }
    /// 清空畫面（當LCD關閉時使用）
    pub fn clear_screen(&mut self) {
        self.framebuffer.fill(0xFFFFFFFF); // 填充白色
                                           // log: PPU: 畫面已清空為白色 (LCD可能已關閉或LCDC寄存器設置有誤)
    }

    #[allow(dead_code)]
    // 獲取瓦片像素顏色的輔助方法
    fn get_tile_pixel_color(
        &self,
        _tile_data: &[u8],
        _x: usize,
        _y: usize,
        _palette: u8,
    ) -> u32 {
        0
    }

    #[allow(dead_code)]
    fn get_color_from_palette(&self, _palette: u8, _color_id: u8) -> u32 {
        0
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
        // 保護位 0-2,只允許設置位 3-7
        let protected_bits = self.stat & 0x07;
        let new_value = (value & 0xF8) | protected_bits;
        self.stat = new_value;
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

    pub fn get_lcdc(&self) -> u8 {
        self.lcdc
    }

    pub fn get_bgp(&self) -> u8 {
        self.bgp
    }

    pub fn get_obp0(&self) -> u8 {
        self.obp0
    }

    pub fn get_obp1(&self) -> u8 {
        self.obp1
    }

    pub fn get_scx(&self) -> u8 {
        self.scx
    }

    pub fn get_scy(&self) -> u8 {
        self.scy
    }

    pub fn get_wx(&self) -> u8 {
        self.wx
    }

    pub fn get_wy(&self) -> u8 {
        self.wy
    }

    /// 獲取當前 PPU 模式
    pub fn get_mode(&self) -> u8 {
        self.mode
    }

    fn scan_oam(&mut self) {
        // OAM掃描階段，蒐集當前掃描線上可見的精靈
        let mut visible_sprites = Vec::with_capacity(10);

        for i in 0..40 {
            let base = i * 4;
            let y_pos = self.oam[base] as i16 - 16;
            let _x_pos = self.oam[base + 1] as i16 - 8;
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
        // 檢查 LCDC 的背景啟用位
        if (self.lcdc & 0x01) == 0 {
            // 如果背景被禁用,填充白色
            for x in 0..160 {
                let fb_index = (self.ly as usize * 160 + x) as usize;
                self.framebuffer[fb_index] = 0xFFFFFFFF;
            }
            return;
        }

        let bg_tile_map = if (self.lcdc & 0x08) != 0 {
            0x1C00 // 使用第二塊瓦片地圖 (0x9C00-0x9FFF)
        } else {
            0x1800 // 使用第一塊瓦片地圖 (0x9800-0x9BFF)
        };

        // 根據 LCDC 選擇瓦片數據區域
        let tile_data = if (self.lcdc & 0x10) != 0 {
            0x0000 // 使用 0x8000-0x8FFF
        } else {
            0x1000 // 使用 0x8800-0x97FF
        };

        let y_pos = (self.ly as u16 + self.scy as u16) & 0xFF;
        let tile_y = (y_pos / 8) as usize;

        // --- DEBUG: 前 10 個像素寫入 logs/debug_ppu.log ---
        if self.ly < 2 {
            // 只記錄前兩條掃描線避免 log 爆量
            let logger = Logger::default();
            for x in 0..10 {
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
                let palette_color = (self.bgp >> (color_num * 2)) & 0x03;
                logger.log_to_file(&format!(
                    "LY={}, X={}, tile_index={:02X}, tile_addr={:04X}, byte1={:02X}, byte2={:02X}, palette_color={}",
                    self.ly, x, tile_index, tile_addr, byte1, byte2, palette_color
                ));
            }
        }
        // --- END DEBUG ---

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
            let color_num = ((byte2 >> color_bit) & 1) << 1 | ((byte1 >> color_bit) & 1); // 獲取調色板顏色編號
            let palette_color = (self.bgp >> (color_num * 2)) & 0x03;

            // 轉換為RGB顏色
            let color = match palette_color {
                0 => 0xFFFFFFFF, // White (00)
                1 => 0xFFC0C0C0, // Light gray (01)
                2 => 0xFF606060, // Dark gray (10)
                3 => 0xFF000000, // Black (11)
                _ => unreachable!(),
            };

            let fb_index = (self.ly as usize * 160 + x) as usize;
            self.framebuffer[fb_index] = color;
        }
    }
    fn render_window(&mut self) {
        // 只有在 WY <= LY 且 WX <= 166 時才渲染窗口
        // 這遵循 Game Boy 硬體的行為，無需警告信息
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

            // 檢查精靈是否在當前掃描線上
            if y_pos > self.ly as i16 || y_pos + sprite_size <= self.ly as i16 {
                continue;
            }

            // 檢查精靈是否在屏幕外
            if x_pos >= 160 || x_pos + 8 <= 0 {
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

    /// 統一的寄存器讀取介面
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.lcdc,
            0xFF41 => self.stat,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF,
        }
    }

    /// 統一的寄存器寫入介面
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF40 => self.set_lcdc(value),
            0xFF41 => self.set_stat(value),
            0xFF42 => self.set_scy(value),
            0xFF43 => self.set_scx(value),
            0xFF45 => self.set_lyc(value),
            0xFF47 => self.set_bgp(value),
            0xFF48 => self.set_obp0(value),
            0xFF49 => self.set_obp1(value),
            0xFF4A => self.set_wy(value),
            0xFF4B => self.set_wx(value),
            _ => {}
        }
    }

    /// 調試函數: 顯示VRAM中的重要數據
    pub fn debug_vram_content(&self) -> String {
        let mut output = String::new();

        // 顯示前16個瓦片數據的第一行
        output.push_str("VRAM 瓦片數據樣本:\n");
        for tile_idx in 0..16 {
            let tile_addr = tile_idx * 16;
            let byte1 = self.vram[tile_addr];
            let byte2 = self.vram[tile_addr + 1];

            output.push_str(&format!(
                "瓦片 {:02X}: {:02X}{:02X} ",
                tile_idx, byte1, byte2
            ));

            // 顯示瓦片第一行的圖案
            for bit in (0..8).rev() {
                let color = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                match color {
                    0 => output.push('□'), // 白色
                    1 => output.push('▒'), // 淺灰
                    2 => output.push('▓'), // 深灰
                    3 => output.push('■'), // 黑色
                    _ => unreachable!(),
                }
            }
            output.push('\n');
        }

        // 顯示背景地圖的一部分
        let bg_map_addr = if (self.lcdc & 0x08) != 0 {
            0x1C00
        } else {
            0x1800
        };
        output.push_str(&format!("\n背景地圖 (位址: 0x{:04X}):\n", bg_map_addr));

        for y in 0..5 {
            // 只顯示前5行
            for x in 0..10 {
                // 每行顯示10個瓦片
                let idx = bg_map_addr + y * 32 + x;
                if idx < 0x2000 {
                    let tile_id = self.vram[idx];
                    output.push_str(&format!("{:02X} ", tile_id));
                }
            }
            output.push('\n');
        }

        output
    }

    /// 調試函數：打印VRAM中某个瓦片的數據
    pub fn debug_tile(&self, tile_idx: usize) -> String {
        let mut output = String::new();

        let tile_addr = tile_idx * 16;
        if tile_addr + 15 >= self.vram.len() {
            return format!("瓦片索引 {} 超出範圍", tile_idx);
        }

        output.push_str(&format!("瓦片 #{} 數據:\n", tile_idx));

        // 每個瓦片有8行，每行2個字節
        for row in 0..8 {
            let byte1 = self.vram[tile_addr + row * 2];
            let byte2 = self.vram[tile_addr + row * 2 + 1];

            output.push_str(&format!("{:02X} {:02X}: ", byte1, byte2));

            // 顯示瓦片的圖案
            for bit in (0..8).rev() {
                let color_id = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                match color_id {
                    0 => output.push('□'), // 白色
                    1 => output.push('▒'), // 淺灰
                    2 => output.push('▓'), // 深灰
                    3 => output.push('■'), // 黑色
                    _ => unreachable!(),
                }
            }
            output.push('\n');
        }

        output
    }

    /// 調試函數：打印VRAM的基本信息
    pub fn debug_vram_info(&self) -> String {
        let mut output = String::new();

        // 檢查前幾個瓦片是否有非零数據
        let mut has_data = false;
        for i in 0..100 {
            for j in 0..16 {
                if self.vram[i * 16 + j] != 0 {
                    has_data = true;
                    break;
                }
            }
            if has_data {
                break;
            }
        }

        output.push_str(&format!(
            "VRAM 數據狀態: {}\n",
            if has_data { "有數據" } else { "空白" }
        ));
        output.push_str(&format!(
            "LCDC: {:02X} (背景開啟: {}, 精靈開啟: {})\n",
            self.lcdc,
            (self.lcdc & 0x01) != 0,
            (self.lcdc & 0x02) != 0
        ));

        // 顯示背景地圖的前幾個項
        let bg_map_addr = if (self.lcdc & 0x08) != 0 {
            0x1C00
        } else {
            0x1800
        };
        output.push_str(&format!(
            "背景地圖地址: 0x{:04X}\n背景地圖前10項: ",
            bg_map_addr
        ));

        for i in 0..10 {
            output.push_str(&format!("{:02X} ", self.vram[bg_map_addr + i]));
        }

        output
    }
    /// 初始化測試圖案到VRAM
    pub fn initialize_test_patterns(&mut self) {
        // log: 🎨 初始化PPU測試圖案...
        // 清空VRAM
        for i in 0..self.vram.len() {
            self.vram[i] = 0;
        }

        // 繪製更多豐富的測試圖案

        // 1. 實心黑色方塊 (瓦片 #0)
        for i in 0..16 {
            self.vram[i] = 0xFF; // 所有像素都是黑色
        }

        // 2. 棋盤格圖案 (瓦片 #1)
        for i in 0..8 {
            self.vram[16 + i * 2] = if i % 2 == 0 { 0xAA } else { 0x55 }; // 10101010/01010101交替
            self.vram[16 + i * 2 + 1] = if i % 2 == 0 { 0x55 } else { 0xAA }; // 01010101/10101010交替
        }

        // 3. 水平條紋 (瓦片 #2)
        for i in 0..8 {
            self.vram[32 + i * 2] = if i % 2 == 0 { 0xFF } else { 0x00 }; // 全黑/全白行
            self.vram[32 + i * 2 + 1] = if i % 2 == 0 { 0xFF } else { 0x00 }; // 全黑/全白行
        }

        // 4. 垂直條紋 (瓦片 #3)
        for i in 0..16 {
            self.vram[48 + i] = 0xAA; // 10101010 垂直條紋
        }

        // 5. 邊框 (瓦片 #4)
        let border_tile_start = 64;
        self.vram[border_tile_start] = 0xFF; // 第一行全黑
        self.vram[border_tile_start + 1] = 0xFF;

        for i in 1..7 {
            self.vram[border_tile_start + i * 2] = 0x81; // 1000 0001
            self.vram[border_tile_start + i * 2 + 1] = 0x81; // 只有邊緣為黑
        }

        self.vram[border_tile_start + 14] = 0xFF; // 最後一行全黑
        self.vram[border_tile_start + 15] = 0xFF;

        // 6. G字型圖案 (瓦片 #5)
        let g_tile_start = 80;
        for i in 0..16 {
            self.vram[g_tile_start + i] = 0;
        }
        self.vram[g_tile_start] = 0x7E; // 第1行: 01111110
        self.vram[g_tile_start + 1] = 0x7E;
        self.vram[g_tile_start + 2] = 0x60; // 第2行: 01100000
        self.vram[g_tile_start + 3] = 0x60;
        self.vram[g_tile_start + 4] = 0x60; // 第3行: 01100000
        self.vram[g_tile_start + 5] = 0x60;
        self.vram[g_tile_start + 6] = 0x60; // 第4行: 01100000
        self.vram[g_tile_start + 7] = 0x60;
        self.vram[g_tile_start + 8] = 0x6E; // 第5行: 01101110
        self.vram[g_tile_start + 9] = 0x6E;
        self.vram[g_tile_start + 10] = 0x66; // 第6行: 01100110
        self.vram[g_tile_start + 11] = 0x66;
        self.vram[g_tile_start + 12] = 0x66; // 第7行: 01100110
        self.vram[g_tile_start + 13] = 0x66;
        self.vram[g_tile_start + 14] = 0x7E; // 第8行: 01111110
        self.vram[g_tile_start + 15] = 0x7E;

        // 7. B字型圖案 (瓦片 #6)
        let b_tile_start = 96;
        for i in 0..16 {
            self.vram[b_tile_start + i] = 0;
        }
        self.vram[b_tile_start] = 0x7E; // 第1行: 01111110
        self.vram[b_tile_start + 1] = 0x7E;
        self.vram[b_tile_start + 2] = 0x66; // 第2行: 01100110
        self.vram[b_tile_start + 3] = 0x66;
        self.vram[b_tile_start + 4] = 0x66; // 第3行: 01100110
        self.vram[b_tile_start + 5] = 0x66;
        self.vram[b_tile_start + 6] = 0x7E; // 第4行: 01111110
        self.vram[b_tile_start + 7] = 0x7E;
        self.vram[b_tile_start + 8] = 0x66; // 第5行: 01100110
        self.vram[b_tile_start + 9] = 0x66;
        self.vram[b_tile_start + 10] = 0x66; // 第6行: 01100110
        self.vram[b_tile_start + 11] = 0x66;
        self.vram[b_tile_start + 12] = 0x66; // 第7行: 01100110
        self.vram[b_tile_start + 13] = 0x66;
        self.vram[b_tile_start + 14] = 0x7E; // 第8行: 01111110
        self.vram[b_tile_start + 15] = 0x7E;

        // 8. 斜條紋圖案 (瓦片 #7)
        let diagonal_tile_start = 112;
        self.vram[diagonal_tile_start] = 0x80; // 10000000
        self.vram[diagonal_tile_start + 1] = 0x80;
        self.vram[diagonal_tile_start + 2] = 0x40; // 01000000
        self.vram[diagonal_tile_start + 3] = 0x40;
        self.vram[diagonal_tile_start + 4] = 0x20; // 00100000
        self.vram[diagonal_tile_start + 5] = 0x20;
        self.vram[diagonal_tile_start + 6] = 0x10; // 00010000
        self.vram[diagonal_tile_start + 7] = 0x10;
        self.vram[diagonal_tile_start + 8] = 0x08; // 00001000
        self.vram[diagonal_tile_start + 9] = 0x08;
        self.vram[diagonal_tile_start + 10] = 0x04; // 00000100
        self.vram[diagonal_tile_start + 11] = 0x04;
        self.vram[diagonal_tile_start + 12] = 0x02; // 00000010
        self.vram[diagonal_tile_start + 13] = 0x02;
        self.vram[diagonal_tile_start + 14] = 0x01; // 00000001
        self.vram[diagonal_tile_start + 15] = 0x01;

        // 設置背景瓦片地圖，創建"Game Boy"字樣和測試圖案

        // 清空背景地圖
        for i in 0x1800..0x1C00 {
            self.vram[i] = 0;
        }

        // 在頂部建立一個邊界
        for i in 0..32 {
            self.vram[0x1800 + i] = 4; // 邊框瓦片
        }

        // 在左右兩側建立邊界
        for i in 1..17 {
            self.vram[0x1800 + i * 32] = 4; // 左邊界
            self.vram[0x1800 + i * 32 + 31] = 4; // 右邊界
        }

        // 在底部建立邊界
        for i in 0..32 {
            self.vram[0x1800 + 17 * 32 + i] = 4; // 底部邊界
        }

        // 在中間放置"GAME BOY"字樣 (使用G和B字母瓦片)
        self.vram[0x1800 + 5 * 32 + 12] = 5; // G
        self.vram[0x1800 + 5 * 32 + 13] = 0; // A (用黑方塊代替)
        self.vram[0x1800 + 5 * 32 + 14] = 2; // M (用條紋代替)
        self.vram[0x1800 + 5 * 32 + 15] = 1; // E (用棋盤代替)

        self.vram[0x1800 + 7 * 32 + 12] = 6; // B
        self.vram[0x1800 + 7 * 32 + 13] = 3; // O (用垂直條紋代替)
        self.vram[0x1800 + 7 * 32 + 14] = 7; // Y (用斜條紋代替)

        // 在區域內隨機放置一些測試瓦片
        for y in 9..16 {
            for x in 5..27 {
                if y % 3 == 0 && x % 4 == 0 {
                    self.vram[0x1800 + y * 32 + x] = ((x + y) % 7) as u8;
                }
            }
        }

        // log: ✅ 豐富的測試圖案初始化完成
    }

    /// 根據 CPU 執行 cycles 推進 PPU 狀態機，並於正確時機呼叫 render_scanline
    pub fn tick(&mut self, cycles: u32, mmu: &mut crate::mmu::MMU) {
        for _ in 0..cycles {
            self.dots += 1;
            if self.ly < 144 {
                // 在可見掃描線期間
                if self.dots == 80 {
                    // OAM Scan 結束，進入 Mode 3
                    self.mode = 3;
                    self.stat = (self.stat & 0xFC) | 0x03;
                } else if self.dots == 252 {
                    // Mode 3 結束，進入 H-Blank
                    self.mode = 0;
                    self.stat = (self.stat & 0xFC) | 0x00;
                    // H-Blank 時渲染本掃描線
                    self.render_scanline();
                    // H-Blank STAT 中斷
                    if (self.stat & 0x08) != 0 {
                        mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
                    }
                }
            }
            if self.dots >= 456 {
                self.dots -= 456;
                self.ly = self.ly.wrapping_add(1);
                mmu.write_byte(0xFF44, self.ly).ok();
                if self.ly == 144 {
                    // 進入 VBlank
                    self.mode = 1;
                    self.stat = (self.stat & 0xFC) | 0x01;
                    // 設置 VBlank 中斷
                    let if_reg = mmu.read_byte_direct(0xFF0F) | 0x01;
                    mmu.write_byte_direct(0xFF0F, if_reg);
                    // VBlank STAT 中斷
                    if (self.stat & 0x10) != 0 {
                        mmu.write_byte_direct(0xFF0F, mmu.read_byte_direct(0xFF0F) | 0x02);
                    }
                } else if self.ly > 153 {
                    // 新一幀開始
                    self.ly = 0;
                    mmu.write_byte(0xFF44, 0).ok();
                    self.mode = 2;
                    self.stat = (self.stat & 0xFC) | 0x02;
                } else if self.ly < 144 {
                    // H-Blank 結束，進入 OAM Scan
                    self.mode = 2;
                    self.stat = (self.stat & 0xFC) | 0x02;
                }
            }
        }
    }
}
