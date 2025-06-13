pub mod lcd_registers;
pub mod mbc;

use self::lcd_registers::LCDRegisters;
use self::mbc::{MBCController, MBCType, create_mbc_controller};
use crate::timer::Timer;

pub const ROM_BANK_SIZE: usize = 16 * 1024; // 16KB
pub const VRAM_SIZE: usize = 8 * 1024; // 8KB
pub const EXTERNAL_RAM_SIZE: usize = 8 * 1024; // 8KB
pub const WRAM_SIZE: usize = 8 * 1024; // 8KB
pub const OAM_SIZE: usize = 160; // 160 bytes
pub const HRAM_SIZE: usize = 127; // 127 bytes

#[derive(Debug, Clone)]
pub struct ROMInfo {
    pub title: String,
    pub mbc_type: MBCType,
    pub rom_size: usize,
    pub ram_size: usize,
}

impl Default for ROMInfo {
    fn default() -> Self {
        Self {
            title: String::new(),
            mbc_type: MBCType::None,
            rom_size: 0,
            ram_size: 0,
        }
    }
}

/// MMU (記憶體管理單元) 結構
pub struct MMU {
    mbc: Box<dyn MBCController>,
    vram: Vec<u8>,
    wram: Vec<u8>,
    oam: Vec<u8>,
    hram: Vec<u8>,
    ie_register: u8,
    pub rom_info: ROMInfo,
    pub cart_rom: Vec<u8>,
    pub timer: Timer,
    lcd_registers: LCDRegisters, // 新增 LCD 寄存器
}

impl MMU {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let rom_info = if rom_data.len() >= 0x150 {
            let mut title = String::new();
            for i in 0x134..=0x143 {
                if rom_data[i] == 0 {
                    break;
                }
                title.push(rom_data[i] as char);
            }

            ROMInfo {
                title,
                mbc_type: MBCType::from_cartridge_type(rom_data[0x147]),
                rom_size: mbc::get_rom_size_bytes(rom_data[0x148]),
                ram_size: mbc::get_ram_size_bytes(rom_data[0x149]),
            }
        } else {
            ROMInfo::default()
        };

        Self {
            mbc: create_mbc_controller(rom_data.clone()),
            vram: vec![0; VRAM_SIZE],
            wram: vec![0; WRAM_SIZE],
            oam: vec![0; OAM_SIZE],
            hram: vec![0; HRAM_SIZE],
            ie_register: 0,
            rom_info,
            cart_rom: rom_data,
            timer: Timer::new(),
            lcd_registers: LCDRegisters::new(),
        }
    }
    pub fn read_byte(&self, addr: u16) -> u8 {
        let value = match addr {
            0x0000..=0x7FFF => {
                let val = self.mbc.read(addr);
                // ROM 啟動區的讀取操作（僅在首次啟動時顯示）
                if addr <= 0x100 && addr == 0x100 {
                    println!("\n══ ROM 引導區啟動 ══");
                    println!("└─ 進入程式區域 (0x0100)");
                }
                val
            }
            0x8000..=0x9FFF => {
                let val = self.vram[addr as usize - 0x8000];
                // // 顯示 VRAM 讀取操作的調試資訊 (目前已註解)
                // if addr < 0x8100 {
                //     println!("讀取 VRAM: 地址=0x{:04X}, 值=0x{:02X}", addr, val);
                // }
                val
            }
            0xA000..=0xBFFF => self.mbc.read(addr),
            0xC000..=0xDFFF => self.wram[addr as usize - 0xC000],
            0xE000..=0xFDFF => self.wram[addr as usize - 0xE000],
            0xFE00..=0xFE9F => self.oam[addr as usize - 0xFE00],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00..=0xFF7F => self.read_io(addr),
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80],
            0xFFFF => self.ie_register,
        };
        value
    }
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => {
                println!("嘗試寫入 ROM: 地址=0x{:04X}, 值=0x{:02X}", addr, value);
                self.mbc.write(addr, value);
            }
            0x8000..=0x9FFF => {
                let vram_addr = (addr - 0x8000) as usize;
                self.vram[vram_addr] = value;

                // 瓦片數據區域 (0x8000-0x97FF)
                if addr >= 0x8000 && addr < 0x9800 {
                    let tile_block = if addr < 0x9000 { 0 } else { 1 };
                    let tile_index = ((addr & 0x0FFF) / 16) as u16;
                    let row_index = ((addr & 0x000F) / 2) as u8;
                    let is_low_bits = (addr & 0x0001) == 0;

                    println!("\n╔═══════════════════════════════════════════════");
                    println!("║           VRAM 寫入 - 瓦片數據               ");
                    println!("╠═══════════════════════════════════════════════");
                    println!(
                        "║ 區域範圍    : 0x{:04X}-0x{:04X} [瓦片塊 {}]",
                        (addr & 0xF800),
                        (addr & 0xF800) + 0x7FF,
                        tile_block
                    );
                    println!("║ 當前地址    : 0x{:04X}", addr);
                    println!("║ 瓦片編號    : {:3} (0x{:02X})", tile_index, tile_index);
                    println!("║ 行號/像素行 : {:2}/{:2}", row_index, row_index * 8);
                    println!(
                        "║ 資料類型    : {}位元組 (0x{:02X})",
                        if is_low_bits { "低" } else { "高" },
                        value
                    );

                    // 如果是高位元組，計算並顯示整行的像素資料
                    if !is_low_bits {
                        let low_byte = self.vram[vram_addr - 1];
                        println!("╟───────────────────────────────────────────────");
                        println!("║ 像素資料");
                        println!("║ ├─ 低位元組: 0x{:02X} = {:08b}", low_byte, low_byte);
                        println!("║ └─ 高位元組: 0x{:02X} = {:08b}", value, value);
                        println!("╟───────────────────────────────────────────────");
                        println!("║ 像素值 & 視覺化");
                        print!("║ 值  : ");
                        for bit in 0..8 {
                            let low_bit = (low_byte >> (7 - bit)) & 0x01;
                            let high_bit = (value >> (7 - bit)) & 0x01;
                            let pixel_value = (high_bit << 1) | low_bit;
                            print!("{} ", pixel_value);
                        }
                        println!();

                        print!("║ 顯示: ");
                        for bit in 0..8 {
                            let low_bit = (low_byte >> (7 - bit)) & 0x01;
                            let high_bit = (value >> (7 - bit)) & 0x01;
                            let pixel_value = (high_bit << 1) | low_bit;
                            let symbol = match pixel_value {
                                0 => "  ",
                                1 => "░░",
                                2 => "▒▒",
                                3 => "██",
                                _ => "??",
                            };
                            print!("{}", symbol);
                        }
                        println!();
                    }
                }
                // 背景地圖區域 (0x9800-0x9FFF)
                else {
                    let map_number = if addr < 0x9C00 { 0 } else { 1 };
                    let base_addr = if map_number == 0 { 0x9800 } else { 0x9C00 };
                    let x = ((addr - base_addr) % 32) as u8;
                    let y = ((addr - base_addr) / 32) as u8;
                    let relative_addr = addr - base_addr;

                    println!("\n╔═══════════════════════════════════════════════");
                    println!("║           VRAM 寫入 - 背景地圖               ");
                    println!("╠═══════════════════════════════════════════════");
                    println!(
                        "║ 地圖編號    : {} (0x{:04X}-0x{:04X})",
                        map_number,
                        base_addr,
                        base_addr + 0x3FF
                    );
                    println!("║ 當前地址    : 0x{:04X}", addr);
                    println!("║ 地圖偏移    : 0x{:03X}", relative_addr);
                    println!(
                        "║ 座標位置    : ({:2}, {:2}) [索引: {:3}]",
                        x,
                        y,
                        y * 32 + x
                    );

                    // 計算實際的像素座標和瓦片來源
                    let pixel_x = x as u16 * 8;
                    let pixel_y = y as u16 * 8;
                    let tile_addr = if value < 128 {
                        0x8000 + (value as u16 * 16)
                    } else {
                        0x8800 + ((value as u16 - 128) * 16)
                    };

                    println!("║ 像素座標    : ({:3}, {:3})", pixel_x, pixel_y);
                    println!("║ 瓦片編號    : 0x{:02X} ({})", value, value);
                    println!(
                        "║ 瓦片數據源  : 0x{:04X}-0x{:04X}",
                        tile_addr,
                        tile_addr + 15
                    );
                }
                println!("╚═══════════════════════════════════════════════");
            }
            0xA000..=0xBFFF => self.mbc.write(addr, value),
            0xC000..=0xDFFF => self.wram[addr as usize - 0xC000] = value,
            0xE000..=0xFDFF => self.wram[addr as usize - 0xE000] = value,
            0xFE00..=0xFE9F => {
                println!("寫入 OAM: 地址=0x{:04X}, 值=0x{:02X}", addr, value);
                self.oam[addr as usize - 0xFE00] = value;
            }
            0xFEA0..=0xFEFF => {}
            0xFF00..=0xFF7F => self.write_io(addr, value),
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80] = value,
            0xFFFF => self.ie_register = value,
        }
    }

    pub fn write_word(&mut self, addr: u16, value: u16) {
        let low = (value & 0xFF) as u8;
        let high = (value >> 8) as u8;
        self.write_byte(addr, low);
        self.write_byte(addr.wrapping_add(1), high);
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr.wrapping_add(1)) as u16;
        (high << 8) | low
    }

    pub fn vram(&self) -> &[u8] {
        &self.vram
    }
    fn read_io(&self, addr: u16) -> u8 {
        match addr {
            // Joypad Input
            0xFF00 => 0xFF, // TODO: Implement joypad input

            // Serial Data Transfer
            0xFF01 => 0xFF,                   // Serial transfer data
            0xFF02 => 0xFF, // Serial transfer control            // Timer and Divider Registers
            0xFF04 => self.timer.read_div(), // Divider Register (DIV)
            0xFF05 => self.timer.read_tima(), // Timer Counter (TIMA)
            0xFF06 => self.timer.read_tma(), // Timer Modulo (TMA)
            0xFF07 => self.timer.read_tac(), // Timer Control (TAC)            // Audio Registers
            0xFF10..=0xFF3F => 0xFF, // TODO: Implement sound registers

            // LCD Control
            0xFF40 => self.lcd_registers.lcdc, // LCD Control (LCDC)
            0xFF41 => self.lcd_registers.stat, // LCD Status (STAT)
            0xFF42 => self.lcd_registers.scy,  // Scroll Y (SCY)
            0xFF43 => self.lcd_registers.scx,  // Scroll X (SCX)
            0xFF44 => self.lcd_registers.ly,   // LCD Y Coordinate (LY)
            0xFF45 => self.lcd_registers.lyc,  // LY Compare (LYC)
            0xFF46 => self.lcd_registers.dma,  // DMA Transfer
            0xFF47 => self.lcd_registers.bgp,  // BG Palette Data (BGP)
            0xFF48 => self.lcd_registers.obp0, // Object Palette 0 (OBP0)
            0xFF49 => self.lcd_registers.obp1, // Object Palette 1 (OBP1)
            0xFF4A => self.lcd_registers.wy,   // Window Y (WY)
            0xFF4B => self.lcd_registers.wx,   // Window X (WX)

            // Interrupt Flags
            0xFF0F => 0xFF, // Interrupt Flag (IF)

            // High RAM
            0xFF80..=0xFFFE => self.hram[addr as usize - 0xFF80],

            // Interrupt Enable Register
            0xFFFF => self.ie_register,

            // 其他未使用的 I/O 位址返回 0xFF
            _ => 0xFF,
        }
    }

    fn write_io(&mut self, addr: u16, value: u8) {
        match addr {
            // Joypad Input
            0xFF00 => (), // TODO: Implement joypad input

            // Serial Data Transfer
            0xFF01 => (),                           // Serial transfer data
            0xFF02 => (), // Serial transfer control            // Timer and Divider Registers
            0xFF04 => self.timer.write_div(value), // DIV (寫入時重置為 0)
            0xFF05 => self.timer.write_tima(value), // Timer Counter (TIMA)
            0xFF06 => self.timer.write_tma(value), // Timer Modulo (TMA)
            0xFF07 => self.timer.write_tac(value), // Timer Control (TAC)

            // Audio Registers
            0xFF10..=0xFF3F => (), // TODO: Implement sound registers            // LCD Control
            0xFF40 => self.lcd_registers.lcdc = value, // LCD Control (LCDC)
            0xFF41 => self.lcd_registers.stat = value, // LCD Status (STAT)
            0xFF42 => self.lcd_registers.scy = value, // Scroll Y (SCY)
            0xFF43 => self.lcd_registers.scx = value, // Scroll X (SCX)            0xFF44 => (),          // LCD Y Coordinate (LY) - Read Only
            0xFF45 => self.lcd_registers.lyc = value, // LY Compare (LYC)
            0xFF46 => {
                self.lcd_registers.dma = value; // DMA Transfer
                self.dma_transfer(value);
            }
            0xFF47 => self.lcd_registers.bgp = value, // BG Palette Data (BGP)
            0xFF48 => self.lcd_registers.obp0 = value, // Object Palette 0 (OBP0)
            0xFF49 => self.lcd_registers.obp1 = value, // Object Palette 1 (OBP1)
            0xFF4A => self.lcd_registers.wy = value,  // Window Y (WY)
            0xFF4B => self.lcd_registers.wx = value,  // Window X (WX)

            // Interrupt Flags
            0xFF0F => (), // Interrupt Flag (IF)

            // 其他未使用的 I/O 位址
            _ => (),
        }
    }
    fn dma_transfer(&mut self, value: u8) {
        let source_addr = (value as u16) << 8;
        println!("\n=== 開始 DMA 傳輸 ===");
        println!("源地址: 0x{:04X}", source_addr);

        // 從指定地址複製 160 bytes 到 OAM
        for i in 0..OAM_SIZE {
            let byte = self.read_byte(source_addr + i as u16);
            self.oam[i] = byte;

            // 每 8 bytes 輸出一次狀態
            if i % 8 == 0 {
                println!("  傳輸進度: {}/160 bytes", i);
            }
        }

        println!("DMA 傳輸完成: 已移動 {} bytes 到 OAM 區域", OAM_SIZE);
        println!("=== DMA 傳輸結束 ===\n");
    }
}
