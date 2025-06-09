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
            framebuffer: vec![0xFFFFFFFF; 160 * 144],
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
    }
    pub fn step(&mut self) {
        // 檢查 LCD 是否啟用 (LCDC 第 7 位)
        if (self.lcdc & 0x80) == 0 {
            // LCD 關閉，清空 framebuffer 為白色
            for pixel in &mut self.framebuffer {
                *pixel = 0xFFFFFFFF; // 白色
            }
            return;
        }

        // 強制啟用背景以進行調試（忽略 LCDC bit 0）
        // 這樣我們可以看到 VRAM 中是否有實際的圖形數據
        let bg_enable = true; // 強制啟用背景進行調試

        // 檢查Window是否啟用 (LCDC 第 5 位)
        let window_enable = (self.lcdc & 0x20) != 0;

        // 背景和 Window 渲染
        for y in 0..144 {
            for x in 0..160 {
                let (tile_id, pixel_x, pixel_y) =
                    if window_enable && y as u8 >= self.wy && x as u8 + 7 >= self.wx {
                        // Window Layer
                        let wx = self.wx.saturating_sub(7);
                        let win_x = (x as i16 - wx as i16).max(0) as usize;
                        let win_y = (y as i16 - self.wy as i16).max(0) as usize;
                        let tile_x = win_x / 8;
                        let tile_y = win_y / 8;
                        let tile_map_addr = 0x1C00 + tile_y * 32 + tile_x; // window map: 0x9C00-0x9FFF
                        let tile_id = self.vram.get(tile_map_addr).copied().unwrap_or(0);
                        let pixel_x = win_x % 8;
                        let pixel_y = win_y % 8;
                        (tile_id, pixel_x, pixel_y)
                    } else if bg_enable {
                        // 背景 Layer
                        let scrolled_x = (x as u8).wrapping_add(self.scx) as usize % 256;
                        let scrolled_y = (y as u8).wrapping_add(self.scy) as usize % 256;
                        let tile_x = (scrolled_x / 8) % 32;
                        let tile_y = (scrolled_y / 8) % 32;
                        let tile_map_addr = (tile_y * 32 + tile_x) % 0x400; // wrap 32x32
                        let tile_id = self.vram.get(0x1800 + tile_map_addr).copied().unwrap_or(0);
                        let pixel_x = scrolled_x % 8;
                        let pixel_y = scrolled_y % 8;
                        (tile_id, pixel_x, pixel_y)
                    } else {
                        // 背景和Window都關閉，顯示白色（調色盤ID 0）
                        (0, 0, 0)
                    };
                let color = if bg_enable || window_enable {
                    let tile_data_addr = (tile_id as usize) * 16 + pixel_y * 2;
                    let low_byte = self.vram.get(tile_data_addr).copied().unwrap_or(0);
                    let high_byte = self.vram.get(tile_data_addr + 1).copied().unwrap_or(0);
                    let bit_pos = 7 - pixel_x;
                    let low_bit = (low_byte >> bit_pos) & 1;
                    let high_bit = (high_byte >> bit_pos) & 1;
                    let color_id = (high_bit << 1) | low_bit;

                    // 使用強制調色板進行調試（如果 BGP 為 0x00）
                    let palette = if self.bgp == 0x00 { 0xE4 } else { self.bgp }; // 強制使用可見的調色板
                    let shade = (palette >> (color_id * 2)) & 0b11;
                    match shade {
                        0 => 0xFFFFFFFF, // 白色
                        1 => 0xFFAAAAAA, // 淺灰
                        2 => 0xFF555555, // 深灰
                        3 => 0xFF000000, // 黑色
                        _ => 0xFF00FF00, // 錯誤顏色（綠色）
                    }
                } else {
                    // 背景和Window都關閉時，顯示白色
                    0xFFFFFFFF
                };
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
                        continue;
                    } // 透明
                    let shade = (self.obp0 >> (color_id * 2)) & 0b11;
                    let color = match shade {
                        0 => 0xFFFFFFFF,
                        1 => 0xFFAAAAAA,
                        2 => 0xFF555555,
                        3 => 0xFF000000,
                        _ => 0xFF00FF00,
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
    }
}
