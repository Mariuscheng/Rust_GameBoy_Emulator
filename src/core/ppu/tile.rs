/*
Game Boy Tile System Implementation
=================================
Handles tile encoding, decoding and management
*/

use std::fmt;

#[derive(Clone)]
/// Represents an 8x8 pixel tile
pub struct Tile {
    data: [u8; 16],  // 16 bytes per tile
}

impl Tile {
    pub fn new(data: [u8; 16]) -> Self {
        Self { data }
    }

    pub fn get_pixel(&self, x: u8, y: u8) -> u8 {
        let low = self.data[(y * 2) as usize];
        let high = self.data[(y * 2 + 1) as usize];

        let bit = 7 - x;
        let l = (low >> bit) & 1;
        let h = (high >> bit) & 1;

        (h << 1) | l
    }
}

/// Implements Debug trait for tile visualization
impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tile [")?;
        for row in 0..8 {
            write!(f, "  ")?;
            let byte1 = self.data[row * 2];
            let byte2 = self.data[row * 2 + 1];
            
            for bit in (0..8).rev() {
                let color = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                match color {
                    0 => write!(f, "□")?, // White
                    1 => write!(f, "▒")?, // Light gray
                    2 => write!(f, "▓")?, // Dark gray
                    3 => write!(f, "■")?, // Black
                    _ => write!(f, "?")?,
                }
            }
            writeln!(f)?;
        }
        write!(f, "]")
    }
}

#[derive(Debug)]
pub struct TileData {
    tiles: Vec<Tile>,
}

impl TileData {
    pub fn new() -> Self {
        Self {
            tiles: Vec::with_capacity(384),  // 384 個圖塊（0x8000-0x9800）
        }
    }

    pub fn get_tile(&self, index: u8) -> Option<&Tile> {
        self.tiles.get(index as usize)
    }

    pub fn set_tile(&mut self, index: u8, tile: Tile) {
        if (index as usize) >= self.tiles.len() {
            self.tiles.resize(index as usize + 1, Tile::new([0; 16]));
        }
        self.tiles[index as usize] = tile;
    }

    pub fn update_tile(&mut self, tile_index: u8, byte_index: u8, value: u8) {
        if let Some(tile) = self.tiles.get_mut(tile_index as usize) {
            tile.data[byte_index as usize] = value;
        }
    }
}
