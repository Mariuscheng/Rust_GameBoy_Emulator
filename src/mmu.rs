// MMU 實現，避免循環依賴
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MBCType {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
}

pub struct MBCController {
    pub mbc_type: MBCType,
    pub rom_bank: u8,
    pub ram_bank: u8,
    pub ram_enabled: bool,
    pub mbc1_mode: u8,
}

impl MBCController {
    pub fn new(mbc_type: MBCType) -> Self {
        Self {
            mbc_type,
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            mbc1_mode: 0,
        }
    }
}

pub struct MMU {
    memory: [u8; 0x10000],
    pub vram: Rc<RefCell<[u8; 0x2000]>>,
    pub oam: Rc<RefCell<[u8; 0xA0]>>,
    pub rom: Vec<u8>,
    pub if_reg: u8,
    pub ie_reg: u8,
    pub mbc: MBCController,
    // 簡化的 joypad 狀態
    pub joypad_state: u8,
}

impl MMU {
    pub fn new_with_vram_oam(
        vram: Rc<RefCell<[u8; 0x2000]>>,
        oam: Rc<RefCell<[u8; 0xA0]>>,
    ) -> Self {
        Self {
            memory: [0; 0x10000],
            vram,
            oam,
            rom: Vec::new(),
            if_reg: 0,
            ie_reg: 0,
            mbc: MBCController::new(MBCType::None),
            joypad_state: 0xFF,
        }
    }

    pub fn load_rom(&mut self, rom_data: Vec<u8>) {
        self.rom = rom_data;
        
        if self.rom.len() > 0x147 {
            let cartridge_type = self.rom[0x147];
            self.mbc.mbc_type = match cartridge_type {
                0x00 => MBCType::None,
                0x01..=0x03 => MBCType::MBC1,
                0x05..=0x06 => MBCType::MBC2,
                0x0F..=0x13 => MBCType::MBC3,
                0x19..=0x1E => MBCType::MBC5,
                _ => MBCType::None,
            };
            
            println!("檢測到卡帶類型: 0x{:02X} -> {:?}", cartridge_type, self.mbc.mbc_type);
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                if self.rom.is_empty() {
                    return 0xFF;
                }
                
                match self.mbc.mbc_type {
                    MBCType::None => {
                        if (addr as usize) < self.rom.len() {
                            self.rom[addr as usize]
                        } else {
                            0xFF
                        }
                    }
                    MBCType::MBC1 => {
                        match addr {
                            0x0000..=0x3FFF => {
                                if (addr as usize) < self.rom.len() {
                                    self.rom[addr as usize]
                                } else {
                                    0xFF
                                }
                            }
                            0x4000..=0x7FFF => {
                                let bank = self.mbc.rom_bank as usize;
                                let real_addr = ((bank * 0x4000) + (addr as usize - 0x4000)) % self.rom.len();
                                self.rom[real_addr]
                            }
                            _ => 0xFF,
                        }
                    }
                    _ => {
                        if (addr as usize) < self.rom.len() {
                            self.rom[addr as usize]
                        } else {
                            0xFF
                        }
                    }
                }
            }
            0x8000..=0x9FFF => {
                let vram_addr = (addr - 0x8000) as usize;
                self.vram.borrow()[vram_addr]
            }
            0xFE00..=0xFE9F => {
                let oam_addr = (addr - 0xFE00) as usize;
                self.oam.borrow()[oam_addr]
            }
            0xFF00 => self.joypad_state,
            0xFF0F => self.if_reg,
            0xFFFF => self.ie_reg,
            _ => self.memory[addr as usize],
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => {
                match self.mbc.mbc_type {
                    MBCType::MBC1 => {
                        match addr {
                            0x0000..=0x1FFF => {
                                self.mbc.ram_enabled = (value & 0x0F) == 0x0A;
                            }
                            0x2000..=0x3FFF => {
                                let bank = value & 0x1F;
                                self.mbc.rom_bank = if bank == 0 { 1 } else { bank };
                            }
                            0x4000..=0x5FFF => {
                                self.mbc.ram_bank = value & 0x03;
                            }
                            0x6000..=0x7FFF => {
                                self.mbc.mbc1_mode = value & 0x01;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            0x8000..=0x9FFF => {
                let vram_addr = (addr - 0x8000) as usize;
                self.vram.borrow_mut()[vram_addr] = value;
            }
            0xFE00..=0xFE9F => {
                let oam_addr = (addr - 0xFE00) as usize;
                self.oam.borrow_mut()[oam_addr] = value;
            }
            0xFF00 => self.joypad_state = value,
            0xFF0F => self.if_reg = value,
            0xFFFF => self.ie_reg = value,
            _ => self.memory[addr as usize] = value,
        }
    }

    pub fn set_joypad(&mut self, value: u8) {
        self.joypad_state = value;
    }

    #[allow(dead_code)]
    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram.borrow()[(addr as usize) % 0x2000]
    }

    #[allow(dead_code)]
    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram.borrow_mut()[(addr as usize) % 0x2000] = value;
    }

    #[allow(dead_code)]
    pub fn vram(&self) -> Vec<u8> {
        self.vram.borrow().to_vec()
    }

    #[allow(dead_code)]
    pub fn oam(&self) -> Vec<u8> {
        self.oam.borrow().to_vec()
    }
}
