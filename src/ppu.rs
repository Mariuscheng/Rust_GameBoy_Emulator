pub struct PPU {
    pub vram: [u8; 0x2000], // 8KB VRAM
    framebuffer: Vec<u32>,  // 160x144 畫面
    pub bgp: u8,            // 背景調色板
    pub obp0: u8,           // sprite palette 0
    pub scx: u8,            // 背景水平滾动
    pub scy: u8,            // 背景垂直滾动
    pub wx: u8,             // Window X
    pub wy: u8,             // Window Y
    pub oam: [u8; 160],     // 40 sprites * 4 bytes
    pub lcdc: u8,           // LCD 控制寄存器
}

impl PPU {
    pub fn new() -> Self {
        let ppu = Self {
            vram: [0; 0x2000],
            framebuffer: vec![0xFFFFFFFFu32; 160 * 144],
            bgp: 0xFC,  // 默认 palette（與MMU初始化值匹配）
            obp0: 0xFF, // sprite palette 預設（與MMU初始化值匹配）
            scx: 0,
            scy: 0,
            wx: 0, // Game Boy 实际画面左上角（與MMU初始化值匹配）
            wy: 0,
            oam: [0; 160],
            lcdc: 0x91, // LCD 控制寄存器初始值（LCD 啟用，與MMU初始化值匹配）
        };

        ppu
    }
    pub fn set_bgp(&mut self, value: u8) {
        self.bgp = value;
    }
    pub fn set_obp0(&mut self, value: u8) {
        self.obp0 = value;
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
    }    pub fn step(&mut self) {
        // 檢查 LCD 是否啟用 (LCDC 第 7 位)
        if (self.lcdc & 0x80) == 0 {
            // LCD 關閉，顯示一個明顯的顏色以便調試
            println!("⚠️ LCD 已關閉 (LCDC: 0x{:02X})", self.lcdc);
            for pixel in &mut self.framebuffer {
                *pixel = 0xFF0000FFu32; // 藍色，明顯指示 LCD 關閉
            }
            return;
        }

        // 檢查背景是否啟用 (LCDC 第 0 位)
        let bg_enable = (self.lcdc & 0x01) != 0;

        // 如果背景關閉，顯示一個明顯的顏色以便調試
        if !bg_enable {
            println!("⚠️ 背景已關閉 (LCDC: 0x{:02X})", self.lcdc);
            for pixel in &mut self.framebuffer {
                *pixel = 0xFFFF0000u32; // 紅色，明顯指示背景關閉
            }
            return;
        }// 背景和 Window 渲染
        for y in 0..144 {
            for x in 0..160 {
                let mut color = 0xFFFFFFFFu32; // 默認白色

                // 檢查Window是否啟用並且在範圍內 (LCDC 第 5 位)
                let window_enable = (self.lcdc & 0x20) != 0;
                let in_window = window_enable && y as u8 >= self.wy && x as u8 + 7 >= self.wx;

                if in_window {
                    // Window Layer - 使用瓦片地圖 1 ($9C00-$9FFF)
                    let wx = self.wx.saturating_sub(7);
                    let win_x = (x as i16 - wx as i16).max(0) as usize;
                    let win_y = (y as i16 - self.wy as i16).max(0) as usize;
                    let tile_x = win_x / 8;
                    let tile_y = win_y / 8;

                    if tile_x < 32 && tile_y < 32 {
                        let tile_map_addr = 0x1C00 + tile_y * 32 + tile_x;
                        let tile_id = self.vram.get(tile_map_addr).copied().unwrap_or(0);
                        let pixel_x = win_x % 8;
                        let pixel_y = win_y % 8;
                        color = self.get_tile_pixel_color(tile_id, pixel_x, pixel_y, self.bgp);
                    }
                } else if bg_enable {
                    // 背景層 - 使用瓦片地圖 0 ($9800-$9BFF)
                    let scrolled_x = (x as u8).wrapping_add(self.scx) as usize;
                    let scrolled_y = (y as u8).wrapping_add(self.scy) as usize;
                    let tile_x = (scrolled_x / 8) % 32;
                    let tile_y = (scrolled_y / 8) % 32;
                    let tile_map_addr = 0x1800 + tile_y * 32 + tile_x;
                    let tile_id = self.vram.get(tile_map_addr).copied().unwrap_or(0);
                    let pixel_x = scrolled_x % 8;
                    let pixel_y = scrolled_y % 8;
                    color = self.get_tile_pixel_color(tile_id, pixel_x, pixel_y, self.bgp);
                }

                let fb_idx = y * 160 + x;
                if fb_idx < self.framebuffer.len() {
                    self.framebuffer[fb_idx] = color;
                }
            }
        }
        // Sprite 渲染（OAM 疊加）
        for i in 0..40 {
            let base = i * 4;
            let y_pos = self.oam[base] as i16 - 16;
            let x_pos = self.oam[base + 1] as i16 - 8;
            let tile_idx = self.oam[base + 2] as usize;
            let attr = self.oam[base + 3];
            let flip_x = (attr & 0x20) != 0;
            let flip_y = (attr & 0x40) != 0;
            // 8x8 sprite
            for py in 0..8 {
                let sy = if flip_y { 7 - py } else { py };
                let screen_y = y_pos + py;
                if screen_y < 0 || screen_y >= 144 {
                    continue;
                }
                for px in 0..8 {
                    let sx = if flip_x { 7 - px } else { px };
                    let screen_x = x_pos + px;
                    if screen_x < 0 || screen_x >= 160 {
                        continue;
                    }
                    let tile_addr = tile_idx * 16 + (sy as usize) * 2;
                    let low = self.vram.get(tile_addr).copied().unwrap_or(0);
                    let high = self.vram.get(tile_addr + 1).copied().unwrap_or(0);
                    let bit = 7 - sx;
                    let lo = (low >> bit) & 1;
                    let hi = (high >> bit) & 1;
                    let color_id = (hi << 1) | lo;
                    if color_id == 0 {
                        continue; // 透明
                    }
                    let shade = (self.obp0 >> (color_id * 2)) & 0b11;
                    let color = match shade {
                        0 => 0xFFFFFFFFu32,
                        1 => 0xFFAAAAAAu32,
                        2 => 0xFF555555u32,
                        3 => 0xFF000000u32,
                        _ => 0xFF00FF00u32,
                    };
                    let idx = (screen_y as usize) * 160 + (screen_x as usize);
                    if idx < self.framebuffer.len() {
                        self.framebuffer[idx] = color;
                    }
                }
            }
        }
    }
    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    } // 獲取瓦片像素顏色的輔助方法
    fn get_tile_pixel_color(
        &self,
        tile_id: u8,
        pixel_x: usize,
        pixel_y: usize,
        palette: u8,
    ) -> u32 {
        // 瓦片數據開始於 VRAM 的 $8000 (0x0000 in vram array)
        let tile_data_addr = (tile_id as usize) * 16 + pixel_y * 2;
        if tile_data_addr + 1 >= self.vram.len() {
            return 0xFFFFFFFFu32; // 如果超出範圍，返回白色
        }

        let low_byte = self.vram[tile_data_addr];
        let high_byte = self.vram[tile_data_addr + 1];

        let bit_pos = 7 - pixel_x;
        let low_bit = (low_byte >> bit_pos) & 1;
        let high_bit = (high_byte >> bit_pos) & 1;
        let color_id = (high_bit << 1) | low_bit; // 添加調試信息 - 只為第一個瓦片的第一個像素
        if tile_id == 1 && pixel_x == 0 && pixel_y == 0 {
            println!(
                "🔍 PPU 調試 - 瓦片 {} 像素 ({}, {}):",
                tile_id, pixel_x, pixel_y
            );
            println!("  瓦片地址: 0x{:04X}", tile_data_addr);
            println!("  低字節: 0x{:02X}, 高字節: 0x{:02X}", low_byte, high_byte);
            println!(
                "  位位置: {}, 低位: {}, 高位: {}",
                bit_pos, low_bit, high_bit
            );
            println!("  色彩 ID: {}, 調色板: 0x{:02X}", color_id, palette);
            let shade = (palette >> (color_id * 2)) & 0b11;
            println!(
                "  最終陰影: {} -> {:08X}",
                shade,
                match shade {
                    0 => 0xFFFFFFFFu32,
                    1 => 0xFFAAAAAAu32,
                    2 => 0xFF555555u32,
                    3 => 0xFF000000u32,
                    _ => 0xFF00FF00u32,
                }
            );
        }

        // 從調色板獲取實際顏色
        let shade = (palette >> (color_id * 2)) & 0b11;
        match shade {
            0 => 0xFFFFFFFFu32, // 白色 (最亮)
            1 => 0xFFAAAAAAu32, // 淺灰
            2 => 0xFF555555u32, // 深灰
            3 => 0xFF000000u32, // 黑色 (最暗)
            _ => 0xFF00FF00u32, // 錯誤顏色（綠色）
        }
    }
}
