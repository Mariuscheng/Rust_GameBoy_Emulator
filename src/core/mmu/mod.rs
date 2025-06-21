use crate::error::{Error, HardwareError, Result};
use crate::interface::input::joypad::Joypad;
use std::fs::OpenOptions;
use std::io::Write;

pub mod lcd_registers;
pub mod mbc;

use lcd_registers::LCDRegisters;

/// Nintendo Logo used for ROM validation
const NINTENDO_LOGO: &[u8; 48] = &[
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

/// Game Boy Memory Management Unit (MMU)
#[derive(Debug, Clone)]
pub struct MMU {
    pub cartridge_rom: Vec<u8>,
    pub work_ram: [u8; 0x2000],              // 8KB work RAM
    pub high_ram: [u8; 0x80],                // 128 bytes high RAM
    pub video_ram: [u8; 0x2000],             // 8KB video RAM
    pub object_attribute_memory: [u8; 0xA0], // Sprite Attribute Table
    pub io_registers: [u8; 0x80],            // I/O registers
    pub interrupt_enable: u8,                // 0xFFFF
    pub interrupt_flags: u8,                 // 0xFF0F
    pub lcd_registers: LCDRegisters,
    pub ly: u8,           // Current scanline
    pub lyc: u8,          // LY Compare
    pub instance_id: u64, // Used to identify different MMU instances
}

impl MMU {
    pub fn new() -> Self {
        println!("MMU::new() - Initialization started");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/mmu_init.log")
        {
            let _ = writeln!(file, "[INFO] MMU::new() - Initialization started");
        }

        let instance_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut mmu = Self {
            cartridge_rom: Vec::new(),
            work_ram: [0; 0x2000],
            high_ram: [0; 0x80],
            video_ram: [0; 0x2000],
            object_attribute_memory: [0; 0xA0],
            io_registers: [0; 0x80],
            interrupt_enable: 0,
            interrupt_flags: 0,
            lcd_registers: LCDRegisters::new(),
            ly: 0,
            lyc: 0,
            instance_id,
        };

        // Initialize basic graphics data into VRAM
        mmu.init_default_graphics();
        println!("MMU::new() - Initialization completed");
        mmu
    }

    pub fn read_byte(&self, address: u16) -> Result<u8> {
        let value = match address {
            // ROM area (0x0000-0x7FFF)
            0x0000..=0x7FFF => {
                if address as usize >= self.cartridge_rom.len() {
                    0xFF
                } else {
                    self.cartridge_rom[address as usize]
                }
            }
            0x8000..=0x9FFF => self.video_ram[address as usize - 0x8000],
            0xA000..=0xBFFF => 0xFF, // External RAM (not implemented yet)
            0xC000..=0xDFFF => self.work_ram[address as usize - 0xC000],
            0xE000..=0xFDFF => self.work_ram[address as usize - 0xE000], // Echo RAM
            0xFE00..=0xFE9F => self.object_attribute_memory[address as usize - 0xFE00],
            0xFEA0..=0xFEFF => 0xFF, // Unused area
            0xFF00..=0xFFFF => self.read_io(address)?,
        };

        // Log VRAM read
        if (0x8000..=0x9FFF).contains(&address) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/vram_read.log")
            {
                let _ = writeln!(
                    file,
                    "VRAM Read - Address: 0x{:04X}, Value: 0x{:02X}",
                    address, value
                );
            }
        }

        Ok(value)
    }

    fn read_io(&self, address: u16) -> Result<u8> {
        let value = match address {
            0xFF00 => 0xFF,          // Joypad (not implemented yet)
            0xFF01..=0xFF02 => 0xFF, // Serial transfer (not implemented)
            0xFF04..=0xFF07 => 0xFF, // Timer (not implemented)
            0xFF10..=0xFF3F => 0xFF, // Sound (not implemented)
            0xFF40..=0xFF4B => self.read_lcd_register(address),
            0xFF4C..=0xFF7F => self.io_registers[(address - 0xFF00) as usize],
            0xFF80..=0xFFFE => self.high_ram[(address - 0xFF80) as usize],
            0xFFFF => self.interrupt_enable,
            _ => 0xFF,
        };
        Ok(value)
    }

    fn read_lcd_register(&self, address: u16) -> u8 {
        match address {
            0xFF40 => self.lcd_registers.lcdc,
            0xFF41 => self.lcd_registers.stat,
            0xFF42 => self.lcd_registers.scy,
            0xFF43 => self.lcd_registers.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF47 => self.lcd_registers.bgp,
            0xFF48 => self.lcd_registers.obp0,
            0xFF49 => self.lcd_registers.obp1,
            0xFF4A => self.lcd_registers.wy,
            0xFF4B => self.lcd_registers.wx,
            _ => 0xFF,
        }
    }

    fn lcd_enabled(&self) -> bool {
        // LCDC bit 7 controls LCD enable/disable
        self.lcd_registers.lcdc & 0x80 != 0
    }
    pub fn write_byte(&mut self, address: u16, value: u8) -> Result<()> {
        // Disable frequent memory write logging for performance
        // Only log critical operations if needed

        match address {
            // ROM area (read-only), writes will be ignored
            0x0000..=0x7FFF => Ok(()),
            0x8000..=0x9FFF => {
                // Check LCD status
                let mode = (self.lcd_registers.stat & 0x03) as u8;
                if mode == 3 {
                    return Err(Error::Hardware(HardwareError::MemoryWrite(
                        "Cannot write to VRAM during LCD mode 3".to_string(),
                    )));
                }

                // VRAM write handling
                if address as usize - 0x8000 < self.video_ram.len() {
                    // Determine if write is allowed:
                    // Check if LCD is enabled and in proper mode
                    let can_write = !self.lcd_enabled() || mode != 3;
                    if can_write {
                        self.video_ram[address as usize - 0x8000] = value;

                        // Temporarily enable VRAM logging to debug ROM graphics
                        if let Ok(mut file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("logs/vram_write.log")
                        {
                            writeln!(file, "VRAM Write: [0x{:04X}] = 0x{:02X}", address, value)
                                .ok();
                        }
                    } else {
                        // VRAM write blocked during mode 3 - don't log for performance
                    }
                }
                Ok(())
            }
            0xA000..=0xBFFF => Ok(()), // External RAM (not implemented yet)
            0xC000..=0xDFFF => {
                self.work_ram[(address - 0xC000) as usize] = value;
                Ok(())
            }
            0xE000..=0xFDFF => {
                // Echo RAM
                self.work_ram[(address - 0xE000) as usize] = value;
                Ok(())
            }
            0xFE00..=0xFE9F => {
                self.object_attribute_memory[(address - 0xFE00) as usize] = value;
                Ok(())
            }
            0xFEA0..=0xFEFF => Ok(()), // Unused area
            0xFF00..=0xFF7F => self.write_io_register(address, value),
            0xFF80..=0xFFFE => {
                self.high_ram[(address - 0xFF80) as usize] = value;
                Ok(())
            }
            0xFFFF => {
                self.interrupt_enable = value;
                Ok(())
            }
        }
    }

    fn write_io_register(&mut self, address: u16, value: u8) -> Result<()> {
        match address {
            0xFF40 => {
                self.lcd_registers.lcdc = value;
                Ok(())
            }
            0xFF41 => {
                self.lcd_registers.stat = value;
                Ok(())
            }
            0xFF42 => {
                self.lcd_registers.scy = value;
                Ok(())
            }
            0xFF43 => {
                self.lcd_registers.scx = value;
                Ok(())
            }
            0xFF44 => Ok(()), // LY is read-only
            0xFF45 => {
                self.lyc = value;
                Ok(())
            }
            0xFF47 => {
                self.lcd_registers.bgp = value;
                Ok(())
            }
            0xFF48 => {
                self.lcd_registers.obp0 = value;
                Ok(())
            }
            0xFF49 => {
                self.lcd_registers.obp1 = value;
                Ok(())
            }
            0xFF4A => {
                self.lcd_registers.wy = value;
                Ok(())
            }
            0xFF4B => {
                self.lcd_registers.wx = value;
                Ok(())
            }
            _ => {
                self.io_registers[(address - 0xFF00) as usize] = value;
                Ok(())
            }
        }
    }

    pub fn init_default_graphics(&mut self) {
        // Clear all VRAM
        self.video_ram.fill(0);

        // Initialize LCD registers with default values
        self.lcd_registers.lcdc = 0; // LCD and PPU off initially
        self.lcd_registers.stat = 0; // Clear status
        self.lcd_registers.scy = 0; // Reset scroll Y
        self.lcd_registers.scx = 0; // Reset scroll X
        self.lcd_registers.ly = 0; // Reset current line
        self.lcd_registers.lyc = 0; // Reset line compare
        self.lcd_registers.bgp = 0xFC; // Default palette (11111100)
        self.lcd_registers.obp0 = 0xFF; // Sprite palette 0
        self.lcd_registers.obp1 = 0xFF; // Sprite palette 1
        self.lcd_registers.wy = 0; // Reset window Y
        self.lcd_registers.wx = 0; // Reset window X

        // Initialize OAM (Sprite Attribute Table)
        self.object_attribute_memory.fill(0);
    }

    pub fn vram(&self) -> &[u8] {
        &self.video_ram
    }
    pub fn load_rom(&mut self, rom_data: &[u8]) -> Result<()> {
        self.cartridge_rom = rom_data.to_vec();

        // Log ROM loading
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/rom_load.log")
        {
            writeln!(
                file,
                "Loading ROM - Size: {} bytes, Title: {}",
                self.cartridge_rom.len(),
                String::from_utf8_lossy(&self.cartridge_rom.get(0x134..0x144).unwrap_or(&[]))
            )?;
        }

        // Initialize basic system state after ROM load
        self.init_system_state()?;

        Ok(())
    }
    /// Initialize basic system state for proper Game Boy operation
    fn init_system_state(&mut self) -> Result<()> {
        // Initialize LCD registers to enable display
        self.lcd_registers.lcdc = 0x91; // Enable LCD, background, and use 8x8 sprites
        self.lcd_registers.stat = 0x02; // Start in OAM scan mode
        self.lcd_registers.scy = 0; // Scroll Y
        self.lcd_registers.scx = 0; // Scroll X        self.lcd_registers.bgp = 0xE4; // Background palette (11100100)
        self.lcd_registers.obp0 = 0xFF; // Sprite palette 0
        self.lcd_registers.obp1 = 0xFF; // Sprite palette 1
        self.lcd_registers.wy = 0; // Window Y
        self.lcd_registers.wx = 0; // Window X

        // Initialize LY and LYC
        self.ly = 0;
        self.lyc = 0;

        // Clear interrupt flags
        self.interrupt_flags = 0;
        self.interrupt_enable = 0;

        // Initialize some I/O registers to reasonable defaults
        self.io_registers[0x00] = 0xFF; // Joypad register

        // Keep VRAM clear - let the game initialize it
        self.video_ram.fill(0);

        // Log system initialization
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/system_init.log")
        {
            writeln!(
                file,
                "System state initialized for game - LCDC: 0x{:02X}, BGP: 0x{:02X}",
                self.lcd_registers.lcdc, self.lcd_registers.bgp
            )?;
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        self.work_ram.fill(0);
        self.high_ram.fill(0);
        self.video_ram.fill(0);
        self.object_attribute_memory.fill(0);
        self.io_registers.fill(0);
        self.interrupt_enable = 0;
        self.interrupt_flags = 0;
        self.lcd_registers = LCDRegisters::new();
        self.ly = 0;
        self.lyc = 0;
    }
    pub fn update_joypad_state(&mut self, joypad: &dyn Joypad) {
        let mut value = 0xFF;

        // Select button state if requested
        if (self.io_registers[0x00] & 0x20) == 0 {
            value &= !(((joypad.is_start_pressed() as u8) << 3)
                | ((joypad.is_select_pressed() as u8) << 2)
                | ((joypad.is_b_pressed() as u8) << 1)
                | (joypad.is_a_pressed() as u8));
        }

        // Select directional state if requested
        if (self.io_registers[0x00] & 0x10) == 0 {
            value &= !(((joypad.is_down_pressed() as u8) << 3)
                | ((joypad.is_up_pressed() as u8) << 2)
                | ((joypad.is_left_pressed() as u8) << 1)
                | (joypad.is_right_pressed() as u8));
        }

        self.io_registers[0x00] = (self.io_registers[0x00] & 0xF0) | (value & 0x0F);
    }

    /// Display the boot animation with Nintendo logo
    pub fn show_boot_sequence(&mut self) -> Result<()> {
        // Copy Nintendo logo data to VRAM tile pattern table
        // Nintendo logo starts at tile $19 (25 decimal)
        let logo_start_tile = 25;
        let vram_start = logo_start_tile * 16; // Each tile is 16 bytes
        for (i, &byte) in NINTENDO_LOGO.iter().enumerate() {
            self.video_ram[vram_start + i] = byte;
        }

        // Set up background tile map
        let bg_map_start = 0x1800; // Background tile map at 0x1800-0x1BFF
        let logo_width = 12; // Logo is 12 tiles wide
        let logo_x = 4; // Position logo 4 tiles from left
        let logo_y = 4; // Position logo 4 tiles from top

        // Place logo tiles in background map
        for i in 0..logo_width {
            let map_pos = bg_map_start + (logo_y * 32 + logo_x + i) as usize;
            self.video_ram[map_pos] = (logo_start_tile + i) as u8;
        }

        // Set up display parameters
        self.lcd_registers.scy = 144; // Start from bottom
        self.lcd_registers.scx = 0; // Center horizontally
        self.lcd_registers.bgp = 0xE4; // Set palette (11100100)
        self.lcd_registers.stat = 0; // Clear LCD status

        // Enable LCD and background
        self.lcd_registers.lcdc = 0x91; // 10010001
                                        // Bit 7 = 1: LCD on
                                        // Bit 6 = 0: Window tile map at 0x9800
                                        // Bit 5 = 0: Window disabled
                                        // Bit 4 = 1: BG tile data at 0x8000
                                        // Bit 3 = 0: BG tile map at 0x9800
                                        // Bit 2 = 0: 8x8 sprites
                                        // Bit 1 = 0: Sprites disabled
                                        // Bit 0 = 1: Background enabled

        Ok(())
    }

    /// Update boot animation scroll position
    pub fn update_boot_animation(&mut self) -> bool {
        if self.lcd_registers.scy > 72 {
            // Target Y position is 72
            self.lcd_registers.scy = self.lcd_registers.scy.saturating_sub(1);

            // Log animation state
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/boot_animation.log")
            {
                let _ = writeln!(
                    file,
                    "Boot animation frame: SCY={}, LCDC={:08b}, STAT={:08b}, BGP={:08b}",
                    self.lcd_registers.scy,
                    self.lcd_registers.lcdc,
                    self.lcd_registers.stat,
                    self.lcd_registers.bgp
                );
            }

            true // Animation still in progress
        } else {
            // Log animation completion
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/boot_animation.log")
            {
                let _ = writeln!(file, "Boot animation completed");
            }
            false // Animation complete
        }
    }
}

impl Default for MMU {
    fn default() -> Self {
        Self::new()
    }
}
