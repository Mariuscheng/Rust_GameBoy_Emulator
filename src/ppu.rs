use std::cell::RefCell;
use std::rc::Rc;

pub struct PPU {
    pub vram: Rc<RefCell<[u8; 0x2000]>>, // 8KB VRAM (共享)
    pub oam: Rc<RefCell<[u8; 0xA0]>>,    // 160 bytes OAM (共享)
    pub lcdc: u8,                        // LCD Control Register (0xFF40)
    pub stat: u8,                        // LCDC Status Register (0xFF41)
    pub scy: u8,                         // Scroll Y (0xFF42)
    pub scx: u8,                         // Scroll X (0xFF43)
    pub ly: u8,                          // LY (0xFF44)
    pub lyc: u8,                         // LYC (0xFF45)
    pub bgp: u8,                         // BG Palette (0xFF47)
    pub obp0: u8,                        // OBJ Palette 0 (0xFF48)
    pub obp1: u8,                        // OBJ Palette 1 (0xFF49)
    pub wy: u8,                          // Window Y (0xFF4A)
    pub wx: u8,                          // Window X (0xFF4B)
    pub framebuffer: [u32; 160 * 144],
    window_line: u8,      // 內部窗口行計數器
    window_enabled: bool, // 窗口是否已經啟用
    #[cfg(debug_assertions)]
    pub debug_mode: bool,
    clock: usize,
}

impl Default for PPU {
    fn default() -> Self {
        PPU {
            vram: Rc::new(RefCell::new([0; 0x2000])),
            oam: Rc::new(RefCell::new([0; 0xA0])),
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC, // 預設 Game Boy palette
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            framebuffer: [0; 160 * 144],
            window_line: 0,
            window_enabled: false,
            #[cfg(debug_assertions)]
            debug_mode: false,
            clock: 0,
        }
    }
}

impl PPU {
    pub fn new(vram: Rc<RefCell<[u8; 0x2000]>>, oam: Rc<RefCell<[u8; 0xA0]>>) -> Self {
        PPU {
            vram,
            oam,
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC, // 預設 Game Boy palette
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            framebuffer: [0; 160 * 144],
            window_line: 0,
            window_enabled: false,
            #[cfg(debug_assertions)]
            debug_mode: false,
            clock: 0,
        }
    }

    pub fn step(&mut self) -> bool {
        // 返回是否應觸發 STAT 中斷
        if self.lcdc & 0x80 == 0 {
            // LCD 關閉時，LY=0，clock=0
            self.ly = 0;
            self.clock = 0;
            return false;
        }

        // 檢測中斷條件變化
        let mut stat_interrupt = false;

        // 累計時鐘
        self.clock += 1; // 每 456 時鐘週期是一條掃描線
        if self.clock >= 456 {
            self.clock = 0;
            self.ly = (self.ly + 1) % 154;

            // 處理窗口行計數器更新
            // 只有當窗口功能啟用且我們在視窗區域時才更新窗口行
            let window_active = (self.lcdc & 0x20) != 0 && self.wx <= 166 && self.ly >= self.wy;

            // 當 LY=0 時重置窗口啟用狀態
            if self.ly == 0 {
                self.window_enabled = false;
            }

            // 當視窗首次顯示時，重置窗口行計數器
            if window_active && !self.window_enabled {
                self.window_line = 0;
                self.window_enabled = true;
            }
            // 如果窗口已啟用且當前在窗口區域，增加窗口行計數
            else if self.window_enabled && window_active {
                self.window_line = self.window_line.wrapping_add(1);
            }

            // 設置 PPU 模式
            if self.ly >= 144 {
                // VBlank (模式 1)
                let new_mode = 0x01;
                let old_mode = self.stat & 0x03;
                self.stat = (self.stat & 0xFC) | new_mode;

                // 模式變為 1 (VBlank) 且啟用了模式 1 中斷
                if old_mode != new_mode && (self.stat & 0x10) != 0 {
                    stat_interrupt = true;
                }
            } else {
                // 設置為 OAM 搜索模式 (模式 2) 初始化新掃描線
                let new_mode = 0x02;
                let old_mode = self.stat & 0x03;
                self.stat = (self.stat & 0xFC) | new_mode;

                // 模式變為 2 (OAM) 且啟用了模式 2 中斷
                if old_mode != new_mode && (self.stat & 0x20) != 0 {
                    stat_interrupt = true;
                }
            }

            // LY=LYC 比較
            let old_lyc = self.stat & 0x04;
            if self.ly == self.lyc {
                self.stat |= 0x04; // 設置 LY=LYC 標誌

                // 如果 LY=LYC 變為真且啟用了 LY=LYC 中斷
                if old_lyc == 0 && (self.stat & 0x40) != 0 {
                    stat_interrupt = true;
                }
            } else {
                self.stat &= !0x04; // 清除 LY=LYC 標誌
            }

            // 當 LCD 啟用且 LY < 144 時，渲染掃描線
            if self.ly < 144 {
                self.render_scanline();
            }
        } else if self.ly < 144 {
            // 可見區域的 PPU 模式變更
            let old_mode = self.stat & 0x03;
            let new_mode = match self.clock {
                0..=80 => {
                    // OAM 搜索 (模式 2)
                    0x02
                }
                81..=252 => {
                    // 像素傳輸 (模式 3)
                    0x03
                }
                _ => {
                    // HBlank (模式 0)
                    0x00
                }
            };

            if old_mode != new_mode {
                self.stat = (self.stat & 0xFC) | new_mode;

                // 檢查是否需要觸發相應的中斷
                match new_mode {
                    0x00 => stat_interrupt = (self.stat & 0x08) != 0, // HBlank 中斷
                    0x02 => stat_interrupt = (self.stat & 0x20) != 0, // OAM 中斷
                    _ => {}
                }
            }
        }

        stat_interrupt
    }

    fn render_scanline(&mut self) {
        // 檢查 LCD 是否啟用
        if self.lcdc & 0x80 == 0 {
            // 填充整行為白色（ARGB格式）
            let line_offset = self.ly as usize * 160;
            for x in 0..160 {
                self.framebuffer[line_offset + x] = 0xFFFFFFFF;
            }
            return;
        }

        // 檢查背景是否啟用
        let render_bg = self.lcdc & 0x01 != 0;

        // 檢查視窗是否啟用
        let render_window = (self.lcdc & 0x20 != 0) && (self.wy <= self.ly); // 如果背景和視窗都禁用，填充整行為白色（ARGB格式）
        if !render_bg && !render_window {
            let line_offset = self.ly as usize * 160;
            for x in 0..160 {
                self.framebuffer[line_offset + x] = 0xFFFFFFFF;
            }
            return;
        }

        // 當前掃描線的開始位置
        let line_offset = self.ly as usize * 160;

        // 渲染背景（如果啟用）
        if render_bg {
            // 計算背景在瓷磚地圖中的 Y 位置
            let bg_y = (self.ly as u16 + self.scy as u16) & 0xFF;
            let tile_y = bg_y / 8; // 瓷磚行
            let pixel_y = bg_y % 8; // 瓷磚內的行

            // 選擇正確的瓷磚地圖地址
            let tile_map_addr = if self.lcdc & 0x08 != 0 {
                0x1C00
            } else {
                0x1800
            };

            // 選擇正確的瓷磚數據區域
            let tile_data_method = (self.lcdc & 0x10) != 0;

            // 對 X 坐標循環
            for screen_x in 0..160 {
                // 計算背景中的實際 X 位置
                let bg_x = (screen_x as u16 + self.scx as u16) & 0xFF;
                let tile_x = bg_x / 8; // 瓷磚列
                let pixel_x = bg_x % 8; // 瓷磚內的列

                // 從瓷磚地圖獲取瓷磚索引
                let map_idx = (tile_y as usize * 32 + tile_x as usize) & 0x3FF;
                let tile_idx_addr = tile_map_addr + map_idx;

                // 安全檢查
                let tile_idx = if tile_idx_addr < 0x2000 {
                    self.vram.borrow()[tile_idx_addr]
                } else {
                    0
                };

                // 根據 LCDC 選擇瓷磚數據地址計算方式
                let tile_addr = if tile_data_method {
                    // 使用無符號尋址 (0x8000-0x8FFF)
                    (tile_idx as u16) * 16
                } else {
                    // 使用有符號尋址 (0x8800-0x97FF)
                    0x1000_u16.wrapping_add((tile_idx as i8 as i16 * 16) as u16)
                };

                // 計算瓷磚數據中的行地址
                let line_addr = tile_addr + (pixel_y as u16 * 2);

                // 讀取該行的瓷磚數據 (兩字節)
                let byte1 = if line_addr < 0x2000 {
                    self.vram.borrow()[line_addr as usize]
                } else {
                    0
                };
                let byte2 = if line_addr + 1 < 0x2000 {
                    self.vram.borrow()[(line_addr + 1) as usize]
                } else {
                    0
                };

                // 獲取特定像素的顏色位 (注意：瓷磚數據是從左到右儲存)
                let color_bit = 7 - (pixel_x as u8);
                let color_lo = (byte1 >> color_bit) & 1;
                let color_hi = (byte2 >> color_bit) & 1;
                let color_idx = (color_hi << 1) | color_lo;

                // 使用 BGP 調色盤獲取實際顏色
                let palette_idx = match color_idx {
                    0 => (self.bgp >> 0) & 0x03,
                    1 => (self.bgp >> 2) & 0x03,
                    2 => (self.bgp >> 4) & 0x03,
                    3 => (self.bgp >> 6) & 0x03,
                    _ => 0, // 不應發生
                }; // 將調色盤索引轉換為 RGB 顏色（ARGB格式）
                let color = match palette_idx {
                    0 => 0xFFFFFFFF, // 白色（ARGB格式：FF=不透明）
                    1 => 0xFFAAAAAA, // 淺灰色
                    2 => 0xFF555555, // 深灰色
                    3 => 0xFF000000, // 黑色
                    _ => 0xFFFF00FF, // 錯誤顏色 (洋紅色)
                };

                // 設置像素顏色
                self.framebuffer[line_offset + screen_x] = color;
            }
        } // 渲染視窗（如果啟用）
        if render_window {
            // 只有當視窗啟用 (LCDC.5)、WX 介於 0~166 之間、且當前掃描線大於等於 WY 時才處理視窗
            if self.lcdc & 0x20 != 0 && self.wx <= 166 && self.ly >= self.wy {
                // 計算視窗在螢幕上的實際位置
                // WX=7 對應螢幕的 X=0，所以需要調整
                // 使用 saturating_sub 確保不會下溢
                let window_x_start = self.wx.saturating_sub(7) as usize; // 使用內部窗口行計數器而非基於屏幕位置的計算
                                                                         // 這更符合 Game Boy 的實際硬件行為
                let window_y = self.window_line as usize;

                // 使用位運算優化計算瓷磚位置和像素偏移
                let window_tile_y = window_y >> 3; // 相當於 window_y / 8
                let window_pixel_y = window_y & 0x07; // 相當於 window_y % 8

                // 選擇正確的視窗瓷磚地圖地址 (LCDC.6)
                let window_tile_map = if self.lcdc & 0x40 != 0 {
                    0x1C00
                } else {
                    0x1800
                };

                // 瓷磚數據區域選擇方式 (LCDC.4)
                let tile_data_method = (self.lcdc & 0x10) != 0;

                // 如果視窗在屏幕上，優化窗口瓷磚行計算
                let tile_row_offset = (window_tile_y & 0x1F) << 5; // 相當於 (window_tile_y % 32) * 32

                // 在視窗區域繪製像素，優化循環
                for screen_x in window_x_start..160 {
                    // 計算視窗中的 X 位置，使用位運算
                    let window_x = screen_x - window_x_start;
                    let window_tile_x = window_x >> 3; // 相當於 window_x / 8
                    let window_pixel_x = window_x & 0x07; // 相當於 window_x % 8

                    // 從視窗瓷磚地圖獲取瓷磚索引，使用位運算確保在範圍內
                    let tile_idx_addr = window_tile_map + tile_row_offset + (window_tile_x & 0x1F);

                    // 提前進行範圍檢查，確保安全存取
                    if tile_idx_addr >= 0x2000 {
                        continue;
                    }

                    // 獲取瓷磚索引
                    let tile_idx = self.vram.borrow()[tile_idx_addr];

                    // 根據 LCDC.4 選擇瓷磚數據地址計算方式
                    let tile_addr = if tile_data_method {
                        // 使用無符號尋址 (0x8000)，使用左移運算優化
                        (tile_idx as u16) << 4 // 相當於 tile_idx * 16
                    } else {
                        // 使用有符號尋址 (0x8800)
                        0x1000_u16.wrapping_add(((tile_idx as i8 as i16) << 4) as u16)
                    };

                    // 計算瓷磚數據中的行地址，使用左移優化
                    let byte_addr = tile_addr.wrapping_add((window_pixel_y << 1) as u16) as usize;

                    // 範圍檢查，確保安全存取
                    if byte_addr + 1 >= 0x2000 {
                        continue;
                    }

                    // 讀取瓷磚數據（兩個位元組）
                    let byte1 = self.vram.borrow()[byte_addr];
                    let byte2 = self.vram.borrow()[byte_addr + 1];

                    // 創建位掩碼，用於提取顏色位
                    let bit_mask = 0x80 >> window_pixel_x;

                    // 透過掩碼高效獲取顏色位
                    let color_lo = if byte1 & bit_mask != 0 { 1 } else { 0 };
                    let color_hi = if byte2 & bit_mask != 0 { 2 } else { 0 };
                    let color_idx = color_lo | color_hi;

                    // 從調色盤獲取灰階索引，使用高效的移位和掩碼操作
                    let palette_shift = color_idx << 1; // 0, 2, 4, or 6
                    let gray_shade = (self.bgp >> palette_shift) & 0x03; // 將灰階值轉換為 RGB 顏色（ARGB格式）
                    let color = match gray_shade {
                        0 => 0xFFFFFFFF, // 白色（ARGB格式：FF=不透明）
                        1 => 0xFFAAAAAA, // 淺灰色
                        2 => 0xFF555555, // 深灰色
                        3 => 0xFF000000, // 黑色
                        _ => 0xFFFF00FF, // 錯誤顏色 (不應發生)
                    };

                    // 設置像素顏色（窗口總是覆蓋背景）
                    self.framebuffer[line_offset + screen_x] = color;
                }
            }
        }

        // 如果啟用精靈，渲染它們
        if self.lcdc & 0x02 != 0 {
            self.render_sprites_for_scanline();
        }
    }

    fn render_sprites_for_scanline(&mut self) {
        // 檢查是否啟用精靈
        if self.lcdc & 0x02 == 0 {
            return; // 精靈禁用
        }

        // 確定精靈高度 (8x8 或 8x16)
        let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };

        // 創建一個陣列來存儲要繪製的精靈，最多有 10 個精靈可以在同一掃描線上顯示
        let mut visible_sprites = Vec::with_capacity(10);

        // 遍歷 OAM 以查找與當前掃描線相交的精靈
        // OAM 中最多有 40 個精靈 (每個精靈佔 4 字節)
        for sprite_idx in 0..40 {
            let oam_idx = sprite_idx * 4;

            // 快速檢查：確保索引不會超出範圍
            if oam_idx + 3 >= self.oam.borrow().len() {
                break;
            }

            // 精靈的 Y 位置（實際 Y 位置减 16）
            let y_pos = self.oam.borrow()[oam_idx] as i32 - 16;

            // 檢查精靈是否在當前掃描線上
            if y_pos <= self.ly as i32 && y_pos + sprite_height as i32 > self.ly as i32 {
                // 獲取精靈的 X 位置（實際 X 位置减 8）
                let x_pos = self.oam.borrow()[oam_idx + 1] as i32 - 8;

                // 只有當精靈至少部分在螢幕上時才考慮它
                if x_pos > -8 && x_pos < 160 {
                    // 獲取剩餘精靈屬性
                    let tile_idx = self.oam.borrow()[oam_idx + 2];
                    let attrs = self.oam.borrow()[oam_idx + 3];

                    // 將精靈添加到可見精靈列表，包括 OAM 索引用於優先級排序
                    visible_sprites.push((x_pos, y_pos, tile_idx, attrs, sprite_idx));

                    // 如果已有 10 個可見精靈，不再添加更多 (Game Boy 硬件限制)
                    if visible_sprites.len() >= 10 {
                        break;
                    }
                }
            }
        }

        // Game Boy 按 OAM 索引排序精靈（OAM 中靠前的精靈優先）
        // 注意：在 Game Boy 上，OAM 索引較小的精靈會繪製在後面，後面的精靈會覆蓋前面的
        // 我們不需要額外排序，因為我們按 OAM 索引順序檢查了精靈

        // 追蹤每個像素在當前掃描線上是否已有精靈（用於優先級處理）
        let mut sprite_drawn = [false; 160];

        // 渲染每個可見精靈（從後到前，這樣較高優先級的精靈會覆蓋較低優先級的精靈）
        // 在 OAM 中靠後的精靈具有更高的優先級
        for &(x_pos, y_pos, tile_idx, attrs, _) in visible_sprites.iter().rev() {
            // 解析屬性
            let behind_bg = attrs & 0x80 != 0; // 精靈在背景後面
            let y_flip = attrs & 0x40 != 0; // Y 翻轉
            let x_flip = attrs & 0x20 != 0; // X 翻轉
            let palette = if attrs & 0x10 != 0 {
                self.obp1
            } else {
                self.obp0
            }; // 調色盤選擇

            // 計算精靈在瓷磚中的行
            let mut line = self.ly as i32 - y_pos;

            // 處理 Y 方向翻轉
            if y_flip {
                line = sprite_height as i32 - 1 - line;
            }

            // 對於 8x16 精靈，需要確定使用哪個瓷磚
            let mut tile_addr: usize;
            if sprite_height == 16 {
                // 8x16 模式: 根據行選擇上半或下半部瓷磚
                // 注意：在 8x16 模式下，最低位被忽略
                let adjusted_tile_idx = tile_idx & 0xFE;

                // 選擇上半部或下半部瓷磚
                if line < 8 {
                    tile_addr = (adjusted_tile_idx as u16 * 16) as usize;
                } else {
                    // 對於下半部，使用下一個瓷磚
                    tile_addr = ((adjusted_tile_idx + 1) as u16 * 16) as usize;
                    line -= 8;
                }
            } else {
                // 8x8 模式: 直接使用提供的瓷磚索引
                tile_addr = (tile_idx as u16 * 16) as usize;
            }

            // 加上行偏移
            tile_addr += line as usize * 2;

            // 安全檢查：確保地址在 VRAM 範圍內
            if tile_addr + 1 >= 0x2000 {
                continue;
            }

            // 讀取瓷磚資料
            let byte1 = self.vram.borrow()[tile_addr];
            let byte2 = self.vram.borrow()[tile_addr + 1];

            // 在螢幕上渲染精靈的整行
            for x_offset in 0..8 {
                // 如果精靈在螢幕外，跳過
                let screen_x = x_pos + if x_flip { 7 - x_offset } else { x_offset };
                if screen_x < 0 || screen_x >= 160 {
                    continue;
                }

                let screen_x = screen_x as usize;

                // 如果此像素已被較高優先級的精靈繪製，跳過
                if sprite_drawn[screen_x] {
                    continue;
                }

                // 獲取像素顏色
                let bit = 7 - (if x_flip { 7 - x_offset } else { x_offset }) % 8;
                let color_idx = ((byte1 >> bit) & 1) | (((byte2 >> bit) & 1) << 1);

                // 顏色 0 是透明的
                if color_idx == 0 {
                    continue;
                }

                // 獲取最終顏色
                let gray_shade = match color_idx {
                    1 => (palette >> 2) & 0x3,
                    2 => (palette >> 4) & 0x3,
                    3 => (palette >> 6) & 0x3,
                    _ => 0,
                }; // 將灰度值轉換為 RGB 顏色（ARGB格式）
                let color = match gray_shade {
                    0 => 0xFFFFFFFF, // 白色（ARGB格式：FF=不透明）
                    1 => 0xFFAAAAAA, // 淺灰色
                    2 => 0xFF555555, // 深灰色
                    3 => 0xFF000000, // 黑色
                    _ => 0xFFFF00FF, // 錯誤顏色
                };

                // 如果精靈應該在背景後面，檢查背景優先級
                if behind_bg {
                    // 獲取該像素的背景顏色索引
                    let bg_idx = self.get_bg_color_index(screen_x, self.ly as usize);

                    // 如果背景顏色不是透明（不是 0），則精靈應該被背景覆蓋
                    if bg_idx != 0 {
                        continue;
                    }
                }

                // 設置像素
                self.framebuffer[(self.ly as usize * 160) + screen_x] = color;

                // 標記此像素已被繪製
                sprite_drawn[screen_x] = true;
            }
        }
    }
    fn get_bg_color_index(&self, screen_x: usize, screen_y: usize) -> u8 {
        // 如果背景禁用，直接返回 0
        if self.lcdc & 0x01 == 0 {
            return 0;
        }

        // 計算背景中的實際位置（使用 wrapping_add 處理溢位）
        // 使用 & 0xFF 代替 % 256，更高效
        let bg_x = (screen_x as u16).wrapping_add(self.scx as u16) & 0xFF;
        let bg_y = (screen_y as u16).wrapping_add(self.scy as u16) & 0xFF;

        // 計算瓷磚坐標和瓷磚內的像素坐標
        // 使用位運算代替除法和取餘運算
        let tile_x = (bg_x >> 3) as usize; // 相當於 bg_x / 8
        let tile_y = (bg_y >> 3) as usize; // 相當於 bg_y / 8
        let pixel_x = (bg_x & 7) as usize; // 相當於 bg_x % 8
        let pixel_y = (bg_y & 7) as usize; // 相當於 bg_y % 8

        // 選擇正確的瓷磚地圖基址 (快取為常數)
        let map_addr_base = if self.lcdc & 0x08 != 0 {
            0x1C00
        } else {
            0x1800
        };

        // 計算瓷磚地圖地址 (使用位運算確保循環)
        // tile_x % 32 等價於 tile_x & 0x1F，tile_y % 32 等價於 tile_y & 0x1F
        let map_addr = map_addr_base + ((tile_y & 0x1F) << 5) + (tile_x & 0x1F);

        // 提前檢查範圍，確保安全存取
        if map_addr >= 0x2000 {
            return 0;
        }

        // 獲取瓷磚索引
        let tile_idx = self.vram.borrow()[map_addr];

        // 選擇瓷磚數據區域並直接計算瓷磚地址
        let tile_addr = if self.lcdc & 0x10 != 0 {
            // 使用無符號尋址 (0x8000)
            (tile_idx as u16) << 4 // 乘以 16，使用左移提高效率
        } else {
            // 使用有符號尋址 (0x8800)
            0x1000_u16.wrapping_add(((tile_idx as i8 as i16) << 4) as u16)
        };

        // 計算瓷磚數據位址，加上行偏移
        // pixel_y * 2 也可以使用左移運算 pixel_y << 1
        let byte_addr = tile_addr.wrapping_add((pixel_y << 1) as u16) as usize;

        // 範圍檢查，確保安全存取
        if byte_addr + 1 >= 0x2000 {
            return 0;
        }

        // 讀取瓷磚數據，一次讀取兩個位元組
        let byte1 = self.vram.borrow()[byte_addr];
        let byte2 = self.vram.borrow()[byte_addr + 1];

        // 計算位運算的位移量
        let bit_mask = 0x80 >> pixel_x; // 創建位掩碼 (代替 7-pixel_x 和逐位右移)

        // 使用掩碼獲取顏色位，然後移到正確位置
        let color_lo = if byte1 & bit_mask != 0 { 1 } else { 0 };
        let color_hi = if byte2 & bit_mask != 0 { 2 } else { 0 };

        // 返回顏色索引 (0-3)
        color_lo | color_hi
    }

    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    #[cfg(debug_assertions)]
    pub fn enable_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    #[cfg(debug_assertions)]
    pub fn dump_lcd_state(&self) {
        println!("LCD 控制 (LCDC): 0x{:02X}", self.lcdc);
        println!("LCD 狀態 (STAT): 0x{:02X}", self.stat);
        println!("當前掃描線 (LY): {}", self.ly);
        println!("LY 比較值 (LYC): {}", self.lyc);
        println!("視窗位置: ({}, {})", self.wx, self.wy);
        println!("滾動位置: ({}, {})", self.scx, self.scy);
        println!(
            "調色板: BGP=0x{:02X}, OBP0=0x{:02X}, OBP1=0x{:02X}",
            self.bgp, self.obp0, self.obp1
        );
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub fn dump_framebuffer_sample(&self) {
        // 顯示前幾行的像素顏色
        println!("=== Framebuffer 樣本 ===");
        for y in 0..3 {
            let line_start = y * 160;
            let line_end = line_start + 10;
            let pixels = &self.framebuffer[line_start..line_end];
            println!("行 {}: {:08X?}", y, pixels);
        }
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub fn dump_sprite_state(&self, sprite_index: usize) {
        if sprite_index >= 40 {
            println!("錯誤：精靈索引必須小於 40");
            return;
        }

        let oam_idx = sprite_index * 4;
        let y_pos = self.oam.borrow()[oam_idx] as i32 - 16;
        let x_pos = self.oam.borrow()[oam_idx + 1] as i32 - 8;
        let tile_idx = self.oam.borrow()[oam_idx + 2];
        let attrs = self.oam.borrow()[oam_idx + 3];

        println!("=== 精靈 #{} 狀態 ===", sprite_index);
        println!("位置: ({}, {})", x_pos, y_pos);
        println!("瓷磚索引: 0x{:02X}", tile_idx);
        println!("屬性: 0x{:02X}", attrs);
        println!("  背景後: {}", if attrs & 0x80 != 0 { "是" } else { "否" });
        println!("  Y翻轉: {}", if attrs & 0x40 != 0 { "是" } else { "否" });
        println!("  X翻轉: {}", if attrs & 0x20 != 0 { "是" } else { "否" });
        println!(
            "  調色盤: {}",
            if attrs & 0x10 != 0 { "OBP1" } else { "OBP0" }
        );
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub fn dump_visible_sprites(&self) {
        println!("=== 當前掃描線 {} 上的可見精靈 ===", self.ly);

        // 確定精靈高度 (8x8 或 8x16)
        let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
        let mut visible_count = 0;

        // 檢查所有精靈
        for sprite_idx in 0..40 {
            let oam_idx = sprite_idx * 4;

            let y_pos = self.oam.borrow()[oam_idx] as i32 - 16;

            // 檢查精靈是否在當前掃描線上
            if y_pos <= self.ly as i32 && y_pos + sprite_height as i32 > self.ly as i32 {
                let x_pos = self.oam.borrow()[oam_idx + 1] as i32 - 8;
                let tile_idx = self.oam.borrow()[oam_idx + 2];
                let attrs = self.oam.borrow()[oam_idx + 3];

                println!(
                    "精靈 #{}: 位置=({}, {}), 瓷磚=0x{:02X}, 屬性=0x{:02X}",
                    sprite_idx, x_pos, y_pos, tile_idx, attrs
                );

                visible_count += 1;
            }
        }

        println!("共 {} 個可見精靈", visible_count);
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub fn dump_tile_maps(&self) {
        println!("=== 瓷磚地圖狀態 ===");
        println!(
            "背景瓷磚地圖地址: 0x{:04X}",
            if self.lcdc & 0x08 != 0 {
                0x9C00
            } else {
                0x9800
            }
        );
        println!(
            "視窗瓷磚地圖地址: 0x{:04X}",
            if self.lcdc & 0x40 != 0 {
                0x9C00
            } else {
                0x9800
            }
        );
        println!(
            "瓷磚數據區域: 0x{:04X}",
            if self.lcdc & 0x10 != 0 {
                0x8000
            } else {
                0x8800
            }
        );

        // 顯示背景瓷磚地圖的一小部分作為示例
        let bg_map_addr = if self.lcdc & 0x08 != 0 {
            0x1C00
        } else {
            0x1800
        };
        println!("背景瓷磚地圖樣本 (左上角4x4區域):");
        for y in 0..4 {
            let mut line = String::new();
            for x in 0..4 {
                let idx = bg_map_addr + y * 32 + x;
                line.push_str(&format!("{:02X} ", self.vram.borrow()[idx]));
            }
            println!("{}", line);
        }
    }
    #[allow(dead_code)]
    pub fn should_trigger_stat_interrupt(&self) -> bool {
        // LCD STAT 寄存器(0xFF41)中的中斷使能位:
        // Bit 6 - LYC=LY 中斷使能
        // Bit 5 - Mode 2 OAM 中斷使能
        // Bit 4 - Mode 1 VBlank 中斷使能
        // Bit 3 - Mode 0 HBlank 中斷使能
        // Bit 2 - LYC=LY 標誌
        // Bit 1-0 - 模式標誌

        let current_mode = self.stat & 0x03;

        // 檢查 LYC=LY 中斷條件
        if (self.stat & 0x04) != 0 && (self.stat & 0x40) != 0 {
            return true;
        }

        // 檢查模式中斷條件
        match current_mode {
            0 => (self.stat & 0x08) != 0, // HBlank 中斷
            1 => (self.stat & 0x10) != 0, // VBlank 中斷
            2 => (self.stat & 0x20) != 0, // OAM 中斷
            _ => false,
        }
    }
}
