use crate::cpu::interrupts::InterruptRegisters;
use crate::error::{Error, Result};
use crate::ppu::PPU;
use crate::timer::Timer;
use std::cell::RefCell;
use std::rc::Rc;

pub mod lcd_registers;
pub mod mbc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MBCType {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
}

impl Default for MBCType {
    fn default() -> Self {
        MBCType::None
    }
}

#[derive(Debug)]
pub struct MMU {
    // ROM 儲存體
    pub rom: Vec<u8>,

    // 工作 RAM (WRAM)
    pub wram: Vec<u8>,

    // 視頻 RAM (VRAM)
    pub vram: Vec<u8>,

    // 精靈資料 (OAM)
    pub oam: Vec<u8>,

    // 外部 RAM (ERAM)
    pub external_ram: Vec<u8>,

    // 高速 RAM (HRAM)
    pub hram: Vec<u8>,

    // I/O 寄存器
    pub io_registers: Vec<u8>,

    // 中斷使能寄存器
    pub ie: u8,

    // 中斷寄存器
    pub if_reg: u8,

    // MBC 控制
    pub mbc_type: MBCType,
    pub rom_bank: u16,
    pub ram_bank: u8,
    pub rom_ram_mode: bool,
    pub ram_enabled: bool,

    // 組件
    pub ppu: Option<Rc<RefCell<PPU>>>,
    pub timer: Option<Rc<RefCell<Timer>>>,
    pub interrupt_registers: Option<Rc<RefCell<InterruptRegisters>>>,

    // 序列介面
    pub serial_data: u8,
    pub serial_control: u8,

    // 輸入按鍵狀態
    pub joypad_state: u8,
}

impl MMU {
    pub fn new(rom: Vec<u8>) -> Self {
        // 檢測 MBC 類型
        let mbc_type = match rom.get(0x147) {
            Some(0x00) => MBCType::None,
            Some(0x01..=0x03) => MBCType::MBC1,
            Some(0x05..=0x06) => MBCType::MBC2,
            Some(0x0F..=0x13) => MBCType::MBC3,
            Some(0x19..=0x1E) => MBCType::MBC5,
            _ => MBCType::None,
        };

        // 初始化 MMU
        MMU {
            rom,
            wram: vec![0; 0x2000],
            vram: vec![0; 0x2000], // 不再預先清空 VRAM
            oam: vec![0; 0xA0],
            external_ram: vec![0; 0x8000],
            hram: vec![0; 0x7F],
            io_registers: vec![0; 0x80],
            ie: 0,
            if_reg: 0,
            mbc_type,
            rom_bank: 1,
            ram_bank: 0,
            rom_ram_mode: false,
            ram_enabled: false,
            ppu: None,
            timer: None,
            serial_data: 0,
            serial_control: 0,
            joypad_state: 0xFF,
            interrupt_registers: None,
        }
    }

    pub fn init_interrupt_registers(
        &mut self,
        interrupt_registers: Rc<RefCell<InterruptRegisters>>,
    ) {
        self.interrupt_registers = Some(interrupt_registers);
    }

    pub fn read_byte(&self, addr: u16) -> Result<u8> {
        match addr {
            0x0000..=0x7FFF => self.read_rom_bank(addr), // ROM Bank 0/N
            0x8000..=0x97FF => {
                // VRAM Tile Data
                let val = self.vram[addr as usize - 0x8000];
                println!("[MMU] VRAM Tile Data Read: {:04X} -> {:02X}", addr, val);
                Ok(val)
            }
            0x9800..=0x9FFF => {
                // VRAM Tile Maps
                let val = self.vram[addr as usize - 0x8000]; // Tile maps are also in VRAM
                println!("[MMU] VRAM Tile Map Read: {:04X} -> {:02X}", addr, val);
                Ok(val)
            }
            0xA000..=0xBFFF => self.read_external_ram(addr), // External RAM
            0xC000..=0xDFFF => Ok(self.wram[addr as usize - 0xC000]), // Work RAM
            0xE000..=0xFDFF => Ok(self.wram[addr as usize - 0xE000]), // Echo RAM
            0xFE00..=0xFE9F => self.read_oam(addr),          // OAM
            0xFEA0..=0xFEFF => Err(Error::Memory("不可訪問的記憶體區域".to_string())), // Unusable
            0xFF00..=0xFF7F => self.read_io_register(addr),  // I/O Registers
            0xFF80..=0xFFFE => Ok(self.hram[addr as usize - 0xFF80]), // High RAM
            0xFFFF => Ok(self.ie),                           // Interrupt Enable Register
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) -> Result<()> {
        match addr {
            0x0000..=0x7FFF => self.handle_bank_switch(addr, value), // ROM Bank 切換
            0x8000..=0x9FFF => {
                // VRAM
                let vram_addr = addr as usize - 0x8000;
                if vram_addr < self.vram.len() {
                    self.vram[vram_addr] = value;
                    if addr >= 0x8000 && addr <= 0x97FF {
                        // Tile Data
                        println!("[MMU] VRAM Tile Data Write: {:04X} <- {:02X}", addr, value);
                    } else if addr >= 0x9800 && addr <= 0x9FFF {
                        // Tile Map
                        println!("[MMU] VRAM Tile Map Write: {:04X} <- {:02X}", addr, value);
                    }
                    Ok(())
                } else {
                    Err(Error::VramInaccessible)
                }
            }
            0xA000..=0xBFFF => self.write_external_ram(addr, value), // External RAM
            0xC000..=0xDFFF => {
                self.wram[addr as usize - 0xC000] = value;
                Ok(())
            } // Work RAM
            0xE000..=0xFDFF => {
                self.wram[addr as usize - 0xE000] = value;
                Ok(())
            } // Echo RAM
            0xFE00..=0xFE9F => self.write_oam(addr, value),          // OAM
            0xFEA0..=0xFEFF => Err(Error::Memory("不可訪問的記憶體區域".to_string())), // Unusable
            0xFF00..=0xFF7F => self.write_io_register(addr, value),  // I/O Registers
            0xFF80..=0xFFFE => {
                self.hram[addr as usize - 0xFF80] = value;
                Ok(())
            } // High RAM
            0xFFFF => {
                self.ie = value;
                Ok(())
            } // Interrupt Enable Register
        }
    }

    #[allow(dead_code)] // 確保屬性在函數定義之前
    fn is_vram_accessible(&self) -> bool {
        if let Some(ppu) = &self.ppu {
            ppu.borrow().is_vram_accessible()
        } else {
            true
        }
    }

    fn is_oam_accessible(&self) -> bool {
        if let Some(ppu) = &self.ppu {
            ppu.borrow().is_oam_accessible()
        } else {
            true
        }
    }

    fn get_eram_addr(&self, addr: u16) -> u32 {
        match self.mbc_type {
            MBCType::MBC1 => {
                if self.rom_ram_mode {
                    ((self.ram_bank as u32) << 13) | ((addr as u32) & 0x1FFF)
                } else {
                    (addr as u32) & 0x1FFF
                }
            }
            MBCType::MBC3 | MBCType::MBC5 => {
                ((self.ram_bank as u32) << 13) | ((addr as u32) & 0x1FFF)
            }
            _ => (addr as u32) & 0x1FFF,
        }
    }

    fn handle_mbc_write(&mut self, addr: u16, value: u8) -> Result<()> {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = match self.mbc_type {
                    MBCType::MBC2 => (value & 0x0F) == 0x0A && addr & 0x100 == 0,
                    _ => (value & 0x0F) == 0x0A,
                };
                Ok(())
            }
            0x2000..=0x3FFF => {
                self.rom_bank = match self.mbc_type {
                    MBCType::MBC1 => {
                        let bank = value & 0x1F;
                        if bank == 0 {
                            1
                        } else {
                            bank as u16
                        }
                    }
                    MBCType::MBC2 => {
                        if addr & 0x100 == 0 {
                            return Ok(());
                        }
                        let bank = value & 0x0F;
                        if bank == 0 {
                            1
                        } else {
                            bank as u16
                        }
                    }
                    MBCType::MBC3 => {
                        let bank = value & 0x7F;
                        if bank == 0 {
                            1
                        } else {
                            bank as u16
                        }
                    }
                    MBCType::MBC5 => value as u16,
                    _ => 1,
                };
                Ok(())
            }
            0x4000..=0x5FFF => {
                match self.mbc_type {
                    MBCType::MBC1 => {
                        let bank = value & 0x03;
                        if self.rom_ram_mode {
                            self.ram_bank = bank;
                        } else {
                            self.rom_bank = (self.rom_bank & 0x1F) | ((bank as u16) << 5);
                        }
                    }
                    MBCType::MBC3 => {
                        self.ram_bank = value & 0x03;
                    }
                    MBCType::MBC5 => {
                        let bank = value & 0x01;
                        self.rom_bank = (self.rom_bank & 0xFF) | ((bank as u16) << 8);
                    }
                    _ => {}
                }
                Ok(())
            }
            0x6000..=0x7FFF => {
                if let MBCType::MBC1 = self.mbc_type {
                    self.rom_ram_mode = (value & 0x01) != 0;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn read_rom_bank(&self, addr: u16) -> Result<u8> {
        // ROM bank 0: 0x0000-0x3FFF
        if addr <= 0x3FFF {
            if let Some(value) = self.rom.get(addr as usize) {
                Ok(*value)
            } else {
                Err(Error::Rom("存取 ROM bank 0 時超出範圍".into()))
            }
        }
        // ROM bank 1+: 0x4000-0x7FFF
        else {
            let bank_offset = self.rom_bank as usize * 0x4000;
            let rom_addr = bank_offset + (addr as usize - 0x4000);
            if let Some(value) = self.rom.get(rom_addr) {
                Ok(*value)
            } else {
                Err(Error::Rom("存取可切換的 ROM bank 時超出範圍".into()))
            }
        }
    }

    pub fn read_vram(&self, addr: u16) -> Result<u8> {
        let vram_addr = addr as usize - 0x8000;
        if vram_addr < self.vram.len() {
            Ok(self.vram[vram_addr])
        } else {
            Err(Error::VramInaccessible)
        }
    }

    pub fn read_external_ram(&self, addr: u16) -> Result<u8> {
        if !self.ram_enabled {
            return Err(Error::Memory("外部 RAM 未啟用".to_string()));
        }
        let eram_addr = self.get_eram_addr(addr) as usize;
        Ok(self.external_ram[eram_addr])
    }

    pub fn read_oam(&self, addr: u16) -> Result<u8> {
        if !self.is_oam_accessible() {
            return Err(Error::OamInaccessible);
        }
        Ok(self.oam[addr as usize - 0xFE00])
    }

    pub fn read_io_register(&self, addr: u16) -> Result<u8> {
        let offset = addr as usize - 0xFF00;
        match addr {
            0xFF40 => {
                // LCDC
                let val = self.io_registers[0x40];
                // self.logger.lock().unwrap().debug(&format!("[MMU] LCDC (0xFF40) Read: {:02X}", val));
                Ok(val)
            }
            0xFF41 => {
                // STAT
                let val = self.io_registers[0x41];
                // self.logger.lock().unwrap().debug(&format!("[MMU] STAT (0xFF41) Read: {:02X}", val));
                Ok(val)
            }
            0xFF42 => Ok(self.io_registers[0x42]), // SCY
            0xFF43 => Ok(self.io_registers[0x43]), // SCX
            0xFF44 => Ok(self.io_registers[0x44]), // LY
            0xFF45 => Ok(self.io_registers[0x45]), // LYC
            0xFF47 => Ok(self.io_registers[0x47]), // BGP
            0xFF48 => Ok(self.io_registers[0x48]), // OBP0
            0xFF49 => Ok(self.io_registers[0x49]), // OBP1
            0xFF4A => Ok(self.io_registers[0x4A]), // WY
            0xFF4B => Ok(self.io_registers[0x4B]), // WX
            _ => Ok(self.io_registers[offset]),
        }
    }

    pub fn handle_bank_switch(&mut self, addr: u16, value: u8) -> Result<()> {
        self.handle_mbc_write(addr, value)
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) -> Result<()> {
        let vram_addr = addr as usize - 0x8000;
        if vram_addr < self.vram.len() {
            self.vram[vram_addr] = value;
            Ok(())
        } else {
            Err(Error::VramInaccessible)
        }
    }

    pub fn write_external_ram(&mut self, addr: u16, value: u8) -> Result<()> {
        if !self.ram_enabled {
            return Err(Error::Memory("外部 RAM 未啟用".to_string()));
        }
        let eram_addr = self.get_eram_addr(addr) as usize;
        self.external_ram[eram_addr] = value;
        Ok(())
    }

    pub fn write_oam(&mut self, addr: u16, value: u8) -> Result<()> {
        if !self.is_oam_accessible() {
            return Err(Error::OamInaccessible);
        }
        self.oam[addr as usize - 0xFE00] = value;
        Ok(())
    }

    pub fn write_io_register(&mut self, addr: u16, value: u8) -> Result<()> {
        let offset = addr as usize - 0xFF00;
        self.io_registers[offset] = value;

        if addr == 0xFF47 {
            // BGP register
            // 考慮使用日誌框架，例如 `log::debug!` 或 `log::info!`
            // 為了簡單起見，這裡使用 println!
            println!("[MMU] BGP (0xFF47) written with value: {:#04X}", value);
        }

        Ok(())
    }

    pub fn set_ppu(&mut self, ppu: Rc<RefCell<PPU>>) {
        self.ppu = Some(ppu);
    }

    pub fn set_timer(&mut self, timer: Rc<RefCell<Timer>>) {
        self.timer = Some(timer);
    }

    pub fn enable_ram(&mut self) {
        self.ram_enabled = true;
    }

    pub fn disable_ram(&mut self) {
        self.ram_enabled = false;
    }

    pub fn handle_mbc1_write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM Enable
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number
                let bank = match value & 0x1F {
                    0 => 1,
                    n => n,
                };
                self.rom_bank = (self.rom_bank & 0x60) | bank as u16;
            }
            0x4000..=0x5FFF => {
                if self.rom_ram_mode {
                    // RAM Bank Number
                    self.ram_bank = value & 0x03;
                } else {
                    // Upper Bits of ROM Bank Number
                    self.rom_bank = (self.rom_bank & 0x1F) | ((value as u16 & 0x03) << 5);
                }
            }
            0x6000..=0x7FFF => {
                // ROM/RAM Mode Select
                self.rom_ram_mode = (value & 0x01) == 0x01;
            }
            _ => {}
        }
    }

    pub fn init_mbc_banks(&mut self) {
        self.rom_bank = 1;
        self.ram_bank = 0;
        self.rom_ram_mode = false;
        self.ram_enabled = false;
    }

    // 添加新的方法來初始化 ROM 區域的圖形數據
    pub fn initialize_tile_data(&mut self) -> Result<()> {
        const TILE_DATA_START: usize = 0x8000;
        const TILE_DATA_END: usize = 0x97FF;
        const VRAM_SIZE: usize = 0x2000; // 8KB VRAM        // 確保不超出 VRAM 範圍
        if TILE_DATA_END - 0x8000 >= VRAM_SIZE {
            return Err(Error::VramInaccessible);
        }

        // 只初始化 VRAM 中的圖形數據範圍，注意減去基礎地址
        for i in 0..=(TILE_DATA_END - TILE_DATA_START) {
            let addr = TILE_DATA_START + i;
            self.write_byte(addr as u16, 0)?;
        }
        Ok(())
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.rom = rom;
        // 根據ROM的類型設置MBC
        self.mbc_type = match self.rom.get(0x147) {
            Some(0x00) => MBCType::None,
            Some(0x01..=0x03) => MBCType::MBC1,
            Some(0x05..=0x06) => MBCType::MBC2,
            Some(0x0F..=0x13) => MBCType::MBC3,
            Some(0x19..=0x1E) => MBCType::MBC5,
            _ => MBCType::None,
        };
    }
}

impl Default for MMU {
    fn default() -> Self {
        MMU {
            rom: Vec::new(),
            wram: vec![0; 0x2000],
            vram: vec![0; 0x2000],
            oam: vec![0; 0xA0],
            external_ram: vec![0; 0x8000],
            hram: vec![0; 0x7F],
            io_registers: vec![0; 0x80],
            ie: 0,
            if_reg: 0,
            mbc_type: MBCType::None,
            rom_bank: 1,
            ram_bank: 0,
            rom_ram_mode: false,
            ram_enabled: false,
            ppu: None,
            timer: None,
            serial_data: 0,
            serial_control: 0,
            joypad_state: 0xFF,
            interrupt_registers: None,
        }
    }
}
