use crate::config::video::VideoConfig;
use crate::ppu::registers::{SCREEN_HEIGHT, SCREEN_WIDTH};
use std::error::Error;

const BGP_REGISTER_ADDRESS: u16 = 0xFF47;

// 視頻記憶體和調色板
#[derive(Debug)]
pub struct Display {
    // 螢幕緩衝區
    pub buffer: Vec<u8>,

    // 調色板
    pub bgp: u8,  // 背景調色板
    pub obp0: u8, // 精靈調色板 0
    pub obp1: u8, // 精靈調色板 1

    // 捲動寄存器
    pub scx: u8, // 背景 X 軸捲動
    pub scy: u8, // 背景 Y 軸捲動
    pub wx: u8,  // 視窗 X 位置
    pub wy: u8,  // 視窗 Y 位置

    // 視訊配置
    config: Option<VideoConfig>,

    // 縮放比例
    #[allow(dead_code)] // 新增 allow(dead_code)
    scale: u32,
}

impl Display {
    pub fn new() -> Self {
        Self {
            buffer: vec![0; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize * 4], // RGBA8888 格式
            bgp: 0xE4,                                                           // 修改為全黑
            obp0: 0xFF,                                                          // 預設精靈調色板 0
            obp1: 0xFF,                                                          // 預設精靈調色板 1
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            config: None,
            scale: 1, // 默認縮放比例為 1
        }
    }

    pub fn init_config(&mut self, config: VideoConfig) {
        self.config = Some(config);
    }

    /// 將顏色 ID 轉換為 RGBA 值
    pub fn palette_color_to_rgba(&self, color_id: u8) -> [u8; 4] {
        // Game Boy 原始調色板 (遵循官方說明文件):
        // 0: 白色 (最亮) - RGB(255, 255, 255)
        // 1: 淺灰     - RGB(192, 192, 192)
        // 2: 深灰     - RGB(96, 96, 96)
        // 3: 黑色 (最暗) - RGB(0, 0, 0)
        match color_id {
            0 => [0xFF, 0xFF, 0xFF, 0xFF], // 白色
            1 => [0xC0, 0xC0, 0xC0, 0xFF], // 淺灰
            2 => [0x60, 0x60, 0x60, 0xFF], // 深灰
            3 => [0x00, 0x00, 0x00, 0xFF], // 黑色
            _ => [0x00, 0x00, 0x00, 0xFF], // 默認黑色
        }
    }

    /// 獲取調色板映射後的顏色
    pub fn get_color(&self, color_id: u8, palette: u8) -> [u8; 4] {
        let palette_color = (palette >> (color_id * 2)) & 0x03;
        self.palette_color_to_rgba(palette_color)
    }

    /// 獲取精靈顏色（包含調色板選擇）
    pub fn get_sprite_color(&self, color_id: u8, use_obp1: bool) -> [u8; 4] {
        let palette = if use_obp1 { self.obp1 } else { self.obp0 };
        self.get_color(color_id, palette)
    }

    /// 設置幀緩衝區中的像素
    pub fn set_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        if x < 160 && y < 144 {
            let offset = (y * 160 + x) * 4;
            self.buffer[offset..offset + 4].copy_from_slice(&color);
        }
    }

    /// 更新指定行的像素
    pub fn update_line(&mut self, line: usize, pixels: &[[u8; 4]]) -> Result<(), Box<dyn Error>> {
        if line >= SCREEN_HEIGHT as usize {
            return Err("Line number out of bounds".into());
        }

        let start = line * SCREEN_WIDTH as usize * 4;
        for (i, pixel) in pixels.iter().enumerate() {
            let base = start + i * 4;
            self.buffer[base..base + 4].copy_from_slice(pixel);
        }

        Ok(())
    }

    /// 清除幀緩衝區（填充黑色）
    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }

    pub fn refresh(&mut self) {
        // 在這裡我們可以添加實際的螢幕刷新邏輯
        // 例如使用 SDL2 或其他圖形庫來顯示緩冲區的內容
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            BGP_REGISTER_ADDRESS => self.bgp,
            // TODO: Implement other PPU register reads if necessary
            _ => 0xFF, // Default for unhandled reads
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            BGP_REGISTER_ADDRESS => {
                // Logger::debug(&format!("[Display] BGP (0xFF47) written with value: {:02X}", value));
                self.bgp = value;
            }
            // TODO: Implement other PPU register writes if necessary
            _ => {} // Default for unhandled writes
        }
    }
}
