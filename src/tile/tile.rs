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
                plane1 |= (color & 1) << (7 - x);
                plane2 |= ((color >> 1) & 1) << (7 - x);
            }

            self.data[y * 2] = plane1;
            self.data[y * 2 + 1] = plane2;
        }
    }

    /// 獲取指定位置的像素顏色 (0-3)
    pub fn get_pixel(&self, x: usize, y: usize) -> u8 {
        self.pixels[y][x]
    }

    /// 設置指定位置的像素顏色 (0-3)
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        self.pixels[y][x] = color & 0x3;
        self.encode();
    }

    /// 獲取原始圖塊數據
    pub fn get_data(&self) -> &[u8; 16] {
        &self.data
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

    /// 旋轉圖塊 (順時針90度)
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
}

/// 實現 Debug trait 用於圖塊可視化
impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tile [")?;
        for row in &self.pixels {
            write!(f, "  ")?;
            for &pixel in row {
                match pixel {
                    0 => write!(f, "□")?, // 白色
                    1 => write!(f, "▒")?, // 淺灰
                    2 => write!(f, "▓")?, // 深灰
                    3 => write!(f, "■")?, // 黑色
                    _ => write!(f, "?")?,
                }
            }
            writeln!(f)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_encoding() {
        // 創建一個簡單的測試圖案
        let mut tile = Tile::new();
        for y in 0..8 {
            for x in 0..8 {
                tile.set_pixel(x, y, ((x + y) % 4) as u8);
            }
        }

        // 編碼後再解碼
        let data = tile.get_data().clone();
        let decoded_tile = Tile::from_data(&data);

        // 驗證像素值
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(tile.get_pixel(x, y), decoded_tile.get_pixel(x, y));
            }
        }
    }

    #[test]
    fn test_tile_flip() {
        let mut tile = Tile::new();
        // 設置一個非對稱圖案
        tile.set_pixel(0, 0, 3);
        tile.set_pixel(1, 0, 2);

        // 測試水平翻轉
        let original = tile.get_pixel(0, 0);
        tile.flip_h();
        assert_eq!(tile.get_pixel(7, 0), original);

        // 測試垂直翻轉
        tile.flip_v();
        assert_eq!(tile.get_pixel(7, 7), original);
    }

    #[test]
    fn test_tile_rotate() {
        let mut tile = Tile::new();
        // 設置一個可識別的圖案
        tile.set_pixel(0, 0, 3);

        // 旋轉90度
        tile.rotate();
        assert_eq!(tile.get_pixel(7, 0), 3);

        // 旋轉180度
        tile.rotate();
        assert_eq!(tile.get_pixel(7, 7), 3);

        // 旋轉270度
        tile.rotate();
        assert_eq!(tile.get_pixel(0, 7), 3);
    }
}
