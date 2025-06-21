//! Background renderer, generates background layer pixels

use crate::core::mmu::MMU;
use crate::core::ppu::registers::{BGP, LCDC, SCX, SCY};
use crate::error::Result;
use std::cell::RefCell;

#[derive(Debug)]
#[allow(dead_code)]
pub struct BackgroundRenderer {
    tile_data: Vec<u8>,
    tile_map: Vec<u8>,
}

impl BackgroundRenderer {
    pub fn new() -> Self {
        Self {
            tile_data: vec![0; 0x1800],
            tile_map: vec![0; 0x800],
        }
    }

    /// Generate background pixel color IDs (0~3) for one scanline, requires MMU to read VRAM/registers
    pub fn render_line(&self, line: u8, mmu: &MMU) -> Result<Vec<u8>> {
        let mut result = vec![0u8; 160]; // Default to white (0)

        // Read PPU related registers
        let lcdc = mmu.read_byte(LCDC)?;

        // If BG display is not enabled, return default color (white)
        if (lcdc & 0x01) == 0 {
            return Ok(result);
        }

        let scx = mmu.read_byte(SCX)?;
        let scy = mmu.read_byte(SCY)?;
        let bgp = mmu.read_byte(BGP)?;
        let vram = mmu.vram(); // Debug info: print register values for first line
                               // Disabled for performance
                               /*
                               if line == 0 {
                                   if let Ok(mut file) = std::fs::OpenOptions::new()
                                       .create(true)
                                       .append(true)
                                       .open("logs/debug_ppu.log")
                                   {
                                       let _ = writeln!(
                                           file,
                                           "PPU accessing MMU instance ID: {}, LCDC: 0x{:02X}, SCX: {}, SCY: {}, BGP: 0x{:02X}",
                                           mmu.instance_id, lcdc, scx, scy, bgp
                                       );
                                       let _ = writeln!(file, "First 16 VRAM bytes: {:02X?}", &vram[0..16]);
                                       let _ = writeln!(file, "Tile map #1 (0x1800): {:02X?}", &vram[0x1800..0x1810]);
                                       let _ = writeln!(file, "Tile map #2 (0x1C00): {:02X?}", &vram[0x1C00..0x1C10]);
                                       let _ = writeln!(file, "First tile data (0x0000): {:02X?}", &vram[0..16]);
                                       let _ = writeln!(
                                           file,
                                           "Background enabled: {}, using tile map #{}, tile data base address: 0x{:04X}",
                                           (lcdc & 0x01) != 0,
                                           if (lcdc & 0x08) != 0 { 2 } else { 1 },
                                           if (lcdc & 0x10) != 0 { 0x0000 } else { 0x1000 }
                                       );
                                   }
                               }
                               */ // Remove duplicate background enable check, as it's already checked above

        // Determine tile map start address
        let bg_tile_map = if (lcdc & 0x08) != 0 { 0x1C00 } else { 0x1800 }; // Determine tile data section (in vram array indices)
        let tile_data_base = if (lcdc & 0x10) != 0 { 0x0000 } else { 0x0800 };

        for x in 0..160u8 {
            let x_pos = (x as u16 + scx as u16) & 0xFF;
            let y_pos = (line as u16 + scy as u16) & 0xFF;
            let tile_x = (x_pos / 8) as usize;
            let tile_y = (y_pos / 8) as usize;
            let tile_map_index = bg_tile_map + tile_y * 32 + tile_x;
            let tile_index = vram[tile_map_index];

            // Calculate tile data address in vram array
            let tile_addr = if (lcdc & 0x10) != 0 {
                tile_data_base + (tile_index as usize * 16)
            } else {
                tile_data_base + ((tile_index as i8 as i16 + 128) as usize * 16)
            };
            let py = (y_pos % 8) as usize;
            let px = (x_pos % 8) as usize;
            let tile_data_addr = tile_addr + py * 2;
            let byte1 = vram.get(tile_data_addr).copied().unwrap_or(0);
            let byte2 = vram.get(tile_data_addr + 1).copied().unwrap_or(0); // Removed tile reading logs for performance
                                                                            /*
                                                                            if line == 0 && x == 0 {
                                                                                if let Ok(mut file) = std::fs::OpenOptions::new()
                                                                                    .create(true)
                                                                                    .append(true)
                                                                                    .open("logs/vram_write.log")
                                                                                {
                                                                                    let _ = writeln!(
                                                                                        file,
                                                                                        "Reading Tile: map_index=0x{:04X}, tile_index={}, tile_data_addr=0x{:04X}, Data=[{:02X},{:02X}]",
                                                                                        tile_map_index, tile_index, tile_data_addr, byte1, byte2
                                                                                    );
                                                                                }
                                                                            }
                                                                            */

            let color_bit = 7 - px;
            let color_num = ((byte2 >> color_bit) & 1) << 1 | ((byte1 >> color_bit) & 1);

            // Convert palette
            let palette_color = (bgp >> (color_num * 2)) & 0x03;
            result[x as usize] = palette_color;
        }

        Ok(result)
    }

    pub fn render_background_line(
        vram: &RefCell<Vec<u8>>,
        framebuffer: &mut [u8],
        line: u8,
        scx: u8,
        scy: u8,
        lcdc: u8,
    ) -> Result<()> {
        let vram = vram.borrow();

        // Determine base address for background tile data
        let tile_data_base = if lcdc & 0x10 != 0 { 0x0000 } else { 0x1000 };

        // Calculate Y coordinate of current line in background map
        let map_y = (line.wrapping_add(scy)) as usize;
        let tile_y = (map_y / 8) % 32;
        let py = map_y % 8;

        // Background map base address (0x9800 or 0x9C00)
        let map_base = if lcdc & 0x08 != 0 { 0x1C00 } else { 0x1800 };

        // Render each pixel in this line
        for screen_x in 0u16..160 {
            // Calculate actual background X coordinate
            let map_x = screen_x.wrapping_add(scx as u16) % 256;
            let tile_x = (map_x / 8) % 32;
            let px = map_x % 8;

            // Calculate tile index position in the background map
            let tile_map_index = map_base + tile_y * 32 + tile_x as usize;
            let tile_index = vram[tile_map_index];

            // Calculate tile data address
            let tile_addr = if lcdc & 0x10 != 0 {
                // 0x8000 addressing mode
                tile_data_base + (tile_index as usize) * 16
            } else {
                // 0x8800 addressing mode
                tile_data_base + ((tile_index as i8 as i16 + 128) as usize) * 16
            };

            // Read the two bytes of tile data
            let byte1 = vram[tile_addr + py * 2];
            let byte2 = vram[tile_addr + py * 2 + 1];

            // Calculate the color value for this pixel
            let color_bit = 7 - (px as u8);
            let color = ((byte2 >> color_bit) & 1) << 1 | ((byte1 >> color_bit) & 1);

            // Write to framebuffer
            let fb_index = line as usize * 160 + screen_x as usize;
            if fb_index < framebuffer.len() {
                framebuffer[fb_index] = color;
            }
        }

        Ok(())
    }
}
