/*
GameBoy 圖塊系統實現
===================
處理圖塊的編碼、解碼和管理
*/

use std::fmt;

/// 代表一個 8x8 像素的圖塊
#[derive(Clone)]
pub struct Tile {
    /// 原始圖塊數據 (16 bytes)
    data: [u8; 16],
    /// 解碼後的像素數據 (8x8 陣列)
    pixels: [[u8; 8]; 8],
}

impl Tile {
    /// 創建一個新的空圖塊
    pub fn new() -> Self {
        Self {
            data: [0; 16],
            pixels: [[0; 8]; 8],
        }
    }

    /// 從原始數據創建圖塊
    pub fn from_data(data: &[u8; 16]) -> Self {
        let mut tile = Self {
            data: *data,
            pixels: [[0; 8]; 8],
        };
        tile.decode();
        tile
    }

    /// 解碼圖塊數據到像素陣列
    fn decode(&mut self) {
        for y in 0..8 {
            let plane1 = self.data[y * 2];
            let plane2 = self.data[y * 2 + 1];

            for x in 0..8 {
                let bit1 = (plane1 >> (7 - x)) & 1;
                let bit2 = (plane2 >> (7 - x)) & 1;
                self.pixels[y][x] = (bit2 << 1) | bit1;
            }
        }
    }

    /// 編碼像素陣列到圖塊數據
    fn encode(&mut self) {
        for y in 0..8 {
            let mut plane1 = 0u8;
            let mut plane2 = 0u8;

            for x in 0..8 {
                let color = self.pixels[y][x];
                plane1 |= ((color & 1) << (7 - x));
                plane2 |= (((color >> 1) & 1) << (7 - x));
            }

            self.data[y * 2] = plane1;
            self.data[y * 2 + 1] = plane2;
        }
    }

    /// 取得特定位置的像素值 (0-3)
    pub fn get_pixel(&self, x: usize, y: usize) -> u8 {
        self.pixels[y][x]
    }

    /// 設置特定位置的像素值 (0-3)
    pub fn set_pixel(&mut self, x: usize, y: usize, value: u8) {
        self.pixels[y][x] = value & 0x3;
        self.encode();
    }

    /// 水平翻轉圖塊
    pub fn flip_h(&mut self) {
        for row in self.pixels.iter_mut() {
            row.reverse();
        }
        self.encode();
    }

    /// 垂直翻轉圖塊
    pub fn flip_v(&mut self) {
        self.pixels.reverse();
        self.encode();
    }

    /// 取得原始圖塊數據
    pub fn get_data(&self) -> &[u8; 16] {
        &self.data
    }

    /// 設置原始圖塊數據
    pub fn set_data(&mut self, data: &[u8; 16]) {
        self.data = *data;
        self.decode();
    }

    /// 旋轉圖塊 (90度順時針)
    pub fn rotate(&mut self) {
        let mut new_pixels = [[0; 8]; 8];
        for y in 0..8 {
            for x in 0..8 {
                new_pixels[x][7 - y] = self.pixels[y][x];
            }
        }
        self.pixels = new_pixels;
        self.encode();
    }

    /// 清除圖塊內容
    pub fn clear(&mut self) {
        self.data = [0; 16];
        self.pixels = [[0; 8]; 8];
    }
}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tile [")?;
        for row in &self.pixels {
            write!(f, "  ")?;
            for &pixel in row {
                write!(f, "{}", pixel)?;
            }
            writeln!(f)?;
        }
        write!(f, "]")
    }
}

/// 圖塊表管理器
pub struct TileMap {
    /// 圖塊數據
    tiles: Vec<Tile>,
    /// 圖塊表模式 (0 或 1)
    mode: u8,
    /// 圖塊表基址
    base_address: u16,
}

impl TileMap {
    /// 創建新的圖塊表
    pub fn new(mode: u8) -> Self {
        let base_address = if mode == 0 { 0x8800 } else { 0x8000 };
        Self {
            tiles: vec![Tile::new(); 256],
            mode,
            base_address,
        }
    }

    /// 設置圖塊數據
    pub fn set_tile(&mut self, index: u8, data: &[u8; 16]) {
        let real_index = if self.mode == 0 {
            (index as i8 as i16 + 128) as usize
        } else {
            index as usize
        };
        self.tiles[real_index] = Tile::from_data(data);
    }

    /// 取得圖塊引用
    pub fn get_tile(&self, index: u8) -> &Tile {
        let real_index = if self.mode == 0 {
            (index as i8 as i16 + 128) as usize
        } else {
            index as usize
        };
        &self.tiles[real_index]
    }

    /// 取得圖塊的可變引用
    pub fn get_tile_mut(&mut self, index: u8) -> &mut Tile {
        let real_index = if self.mode == 0 {
            (index as i8 as i16 + 128) as usize
        } else {
            index as usize
        };
        &mut self.tiles[real_index]
    }

    /// 取得基址
    pub fn get_base_address(&self) -> u16 {
        self.base_address
    }

    /// 設置模式
    pub fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
        self.base_address = if mode == 0 { 0x8800 } else { 0x8000 };
    }

    /// 更新 VRAM 中的圖塊數據
    pub fn update_from_vram(&mut self, vram: &[u8]) {
        let start_addr = (self.base_address - 0x8000) as usize;
        for i in 0..256 {
            let tile_addr = start_addr + i * 16;
            let mut tile_data = [0u8; 16];
            tile_data.copy_from_slice(&vram[tile_addr..tile_addr + 16]);
            self.set_tile(i as u8, &tile_data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_encoding() {
        let data = [
            0b11000011, 0b00111100, // 第一行的兩個平面
            0, 0, 0, 0, 0, 0, 0, 0, // 其餘行
            0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let tile = Tile::from_data(&data);
        assert_eq!(tile.get_pixel(0, 0), 3); // 11
        assert_eq!(tile.get_pixel(1, 0), 0); // 00
        assert_eq!(tile.get_pixel(2, 0), 0); // 00
        assert_eq!(tile.get_pixel(3, 0), 1); // 01
        assert_eq!(tile.get_pixel(4, 0), 1); // 01
        assert_eq!(tile.get_pixel(5, 0), 0); // 00
        assert_eq!(tile.get_pixel(6, 0), 0); // 00
        assert_eq!(tile.get_pixel(7, 0), 3); // 11
    }

    #[test]
    fn test_tile_flipping() {
        let mut tile = Tile::new();
        // 設置一個簡單的圖案
        tile.set_pixel(0, 0, 3);
        tile.set_pixel(7, 7, 2);

        // 測試水平翻轉
        tile.flip_h();
        assert_eq!(tile.get_pixel(7, 0), 3);
        assert_eq!(tile.get_pixel(0, 7), 2);

        // 測試垂直翻轉
        tile.flip_v();
        assert_eq!(tile.get_pixel(7, 7), 3);
        assert_eq!(tile.get_pixel(0, 0), 2);
    }
}
