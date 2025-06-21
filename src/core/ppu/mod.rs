// PPU core module

pub mod background;
pub mod display;
pub mod lcd;
pub mod pixel;
pub mod registers;
pub mod sprite;
pub mod tile;
pub mod types;
pub mod window;

pub(crate) use background::*;
pub(crate) use display::*;
pub(crate) use sprite::*;
pub(crate) use window::*;

use crate::core::mmu::MMU;
use crate::error::Error;
use crate::interface::video::VideoInterface;
use std::cell::RefCell;
use std::rc::Rc;

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;
const VBLANK_LINE: u8 = 144;
const MAX_LINE: u8 = 153;

#[derive(Debug)]
pub struct PPU {
    pub background: BackgroundRenderer,
    pub window: WindowRenderer,
    pub sprites: SpriteRenderer,
    pub display: Display,
    mmu: Rc<RefCell<MMU>>,
    video: Box<dyn VideoInterface>,

    // PPU state
    mode_clock: u32,
    current_line: u8,
    current_mode: u8,
}

impl PPU {
    pub fn new(mmu: Rc<RefCell<MMU>>, video: Box<dyn VideoInterface>) -> Self {
        Self {
            background: BackgroundRenderer::new(),
            window: WindowRenderer::new(),
            sprites: SpriteRenderer::new(),
            display: Display::new(),
            mmu,
            video,
            mode_clock: 0,
            current_line: 0,
            current_mode: 0,
        }
    }

    pub fn render(&mut self) -> crate::error::Result<()> {
        // Update display using video interface
        self.video.update_frame(self.display.get_buffer());
        self.video.render()
    }

    pub fn render_line(&mut self) -> Result<(), Error> {
        let current_line = self.current_line;
        let bg_line = {
            let mmu = self.mmu.borrow();
            self.background.render_line(current_line, &mmu)?
        };

        // Update display buffer
        let base_index = current_line as usize * SCREEN_WIDTH;
        for x in 0..SCREEN_WIDTH {
            let color_id = bg_line[x];
            self.display.get_frame_mut()[base_index + x] = match color_id {
                0 => 0xFFFFFFFF, // White
                1 => 0xFFAAAAAA, // Light gray
                2 => 0xFF555555, // Dark gray
                3 => 0xFF000000, // Black
                _ => 0xFF000000, // Default to black
            };
        }

        Ok(())
    }

    pub fn get_video_mut(&mut self) -> &mut dyn VideoInterface {
        self.video.as_mut()
    }
    pub fn step(&mut self, cycles: u32) -> Result<(), Error> {
        // Check if LCD is enabled
        let lcd_enabled = {
            let mmu = self.mmu.borrow();
            let lcdc = mmu.read_byte(0xFF40).unwrap_or(0);
            (lcdc & 0x80) != 0
        };

        if !lcd_enabled {
            // When LCD is disabled
            self.display.clear();
            self.current_mode = 0;
            self.current_line = 0;
            self.mode_clock = 0;
            self.update_lcd_status()?;
            return Ok(());
        }

        self.mode_clock += cycles;
        let old_mode = self.current_mode;

        match self.current_mode {
            0 => {
                // H-Blank (204 cycles)
                if self.mode_clock >= 204 {
                    self.mode_clock = 0;
                    self.current_line += 1;

                    if self.current_line == VBLANK_LINE {
                        // Enter V-Blank
                        self.current_mode = 1;
                        let mut mmu = self.mmu.borrow_mut();
                        mmu.interrupt_flags |= 1 << 0; // Set VBlank interrupt
                        drop(mmu);

                        // Frame rendering complete, update display
                        self.vblank()?;
                    } else {
                        // Return to OAM scan
                        self.current_mode = 2;
                    }
                }
            }
            1 => {
                // V-Blank (4560 cycles total, 10 lines * 456)
                if self.mode_clock >= 456 {
                    self.mode_clock = 0;
                    self.current_line += 1;

                    if self.current_line > MAX_LINE {
                        // V-Blank ends, return to first line
                        self.current_mode = 2;
                        self.current_line = 0;
                    }
                }
            }
            2 => {
                // OAM scan (80 cycles)
                if self.mode_clock >= 80 {
                    self.mode_clock = 0;
                    self.current_mode = 3;
                }
            }
            3 => {
                // Transfer data to LCD (172 cycles)
                if self.mode_clock >= 172 {
                    self.mode_clock = 0;
                    self.current_mode = 0; // Enter H-Blank

                    // Render current line
                    if self.current_line < SCREEN_HEIGHT as u8 {
                        self.render_line()?;
                    }
                }
            }
            _ => unreachable!(),
        } // If mode changed, update LCD status
        if old_mode != self.current_mode {
            self.update_lcd_status()?;

            // Log mode change - DISABLED FOR PERFORMANCE
            /*
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/debug_ppu.log")
            {
                writeln!(
                    file,
                    "LCD mode change: {} -> {}, line: {}, clock: {}",
                    old_mode, self.current_mode, self.current_line, self.mode_clock
                )
                .ok();
            }
            */
        }

        Ok(())
    }

    pub fn update(&mut self, cycles: u32) -> Result<(), Error> {
        self.step(cycles)
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.mode_clock = 0;
        self.current_line = 0;
        self.current_mode = 0;
        self.display.clear();
        Ok(())
    }

    pub fn get_line(&self) -> u8 {
        self.current_line
    }

    pub fn get_mode(&self) -> u8 {
        self.current_mode
    }
    #[allow(dead_code)]
    fn draw_line(&mut self, line: u8, mmu: &MMU) -> Result<(), Error> {
        // Get background layer
        let bg_line = self.background.render_line(line, mmu)?;

        let base_index = line as usize * SCREEN_WIDTH;
        for x in 0..SCREEN_WIDTH {
            let color_id = bg_line[x];
            self.display.get_frame_mut()[base_index + x] = match color_id {
                0 => 0xFFFFFFFF, // White
                1 => 0xFFAAAAAA, // Light gray
                2 => 0xFF555555, // Dark gray
                3 => 0xFF000000, // Black
                _ => 0xFF000000, // Default to black
            };
        }
        Ok(())
    }

    fn render_current_frame(&mut self) -> Result<(), Error> {
        // Clear entire screen
        self.display.clear();

        // Render each line
        for line in 0..SCREEN_HEIGHT {
            let line = line as u8;

            // First get background line data
            let bg_line = {
                let mmu = self.mmu.borrow();
                self.background.render_line(line, &mmu)?
            };

            // Then update display buffer
            let base_index = line as usize * SCREEN_WIDTH;
            for x in 0..SCREEN_WIDTH {
                let color_id = bg_line[x];
                self.display.get_frame_mut()[base_index + x] = match color_id {
                    0 => 0xFFFFFFFF, // White
                    1 => 0xFFAAAAAA, // Light gray
                    2 => 0xFF555555, // Dark gray
                    3 => 0xFF000000, // Black
                    _ => 0xFF000000, // Default to black
                };
            }
        }

        Ok(())
    }

    fn vblank(&mut self) -> Result<(), Error> {
        // Update screen during V-Blank
        self.render_current_frame()?;
        self.display.render(&mut self.video)
    }

    fn update_lcd_status(&mut self) -> Result<(), Error> {
        let mut mmu = self.mmu.borrow_mut();
        let mut stat = mmu.read_byte(0xFF41)?;

        // Clear current mode bits (0-1)
        stat &= 0xFC;
        // Set new mode
        stat |= self.current_mode & 0x03;

        // Update LYC=LY comparison flag (bit 2)
        let lyc = mmu.read_byte(0xFF45)?;
        if self.current_line == lyc {
            stat |= 0x04;
        } else {
            stat &= !0x04;
        }

        // Write updated STAT
        mmu.write_byte(0xFF41, stat)?;

        // Update LY register
        mmu.write_byte(0xFF44, self.current_line)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ppu_initialization() {
        // TODO: Implement tests
    }
}
