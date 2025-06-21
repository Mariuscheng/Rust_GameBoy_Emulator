//! Display module, manages palette and framebuffer

use super::background::BackgroundRenderer;
use crate::core::mmu::MMU;

#[derive(Debug)]
pub struct Display {
    framebuffer: Vec<u32>,
    color_map: Vec<u32>,
}

impl Display {
    pub fn new() -> Self {
        let color_map = vec![
            0xFFFFFFFF, // White (0)
            0xFFAAAAAA, // Light gray (1)
            0xFF555555, // Dark gray (2)
            0xFF000000, // Black (3)
        ];
        Self {
            framebuffer: vec![0xFFFFFFFF; 160 * 144], // 32-bit RGBA, default to white
            color_map,
        }
    }
    pub fn clear(&mut self) {
        self.framebuffer.fill(0xFFFFFFFF); // Clear to white
    }
    pub fn render(
        &mut self,
        video: &mut Box<dyn crate::interface::video::VideoInterface>,
    ) -> crate::error::Result<()> {
        video.render().map_err(|_| {
            crate::error::Error::Hardware(crate::error::HardwareError::PPU("Rendering failed".to_string()))
        })?;
        Ok(())
    }

    pub fn get_frame(&self) -> &[u32] {
        &self.framebuffer
    }

    pub fn get_frame_mut(&mut self) -> &mut [u32] {
        &mut self.framebuffer
    }

    pub fn update_line(&mut self, line: usize, data: &[u8]) {
        let start = line * 160;
        for (i, &pixel) in data.iter().enumerate().take(160) {
            self.framebuffer[start + i] = self.color_map[pixel as usize & 3];
        }
    }

    pub fn render_game_frame(
        &mut self,
        bg: &BackgroundRenderer,
        mmu: &MMU,
    ) -> crate::error::Result<()> {
        for y in 0..144u8 {
            let line = bg.render_line(y, mmu)?;
            for x in 0..160u8 {
                let color_id = line[x as usize];
                // Convert to grayscale colors according to Game Boy palette
                let gray = match color_id {
                    0 => 0xFFFFFFFF,
                    1 => 0xFFAAAAAA,
                    2 => 0xFF555555,
                    _ => 0xFF000000,
                };
                self.framebuffer[(y as usize) * 160 + (x as usize)] = gray;
            }
        }
        Ok(())
    }

    pub fn set_pixel(
        &mut self,
        x: usize,
        y: usize,
        rgba: [u8; 4],
    ) -> Result<(), crate::error::Error> {
        if x < 160 && y < 144 {
            let index = y * 160 + x;
            let color = ((rgba[3] as u32) << 24)
                | ((rgba[0] as u32) << 16)
                | ((rgba[1] as u32) << 8)
                | (rgba[2] as u32);
            self.framebuffer[index] = color;
        }
        Ok(())
    }

    pub fn present(&mut self) -> Result<(), crate::error::Error> {
        // This method will be handled at the VideoInterface layer
        Ok(())
    }
    /// Get byte slice of current framebuffer
    pub fn get_buffer(&self) -> Vec<u8> {
        // Safely convert u32 to u8 using bytemuck
        self.framebuffer
            .iter()
            .flat_map(|&pixel| pixel.to_ne_bytes())
            .collect()
    }
}
