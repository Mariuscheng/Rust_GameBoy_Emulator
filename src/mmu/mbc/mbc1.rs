use super::{MBCController, types::*};

pub struct MBC1 {
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    state: MBCState,
    rom_bank_count: u16,
    ram_bank_count: u8,
}

impl MBC1 {
    pub fn new(rom_data: Vec<u8>, ram_size: usize, rom_banks: u16, ram_banks: u8) -> Self {
        Self {
            rom_data,
            ram_data: vec![0; ram_size],
            state: MBCState::new(MBCType::MBC1),
            rom_bank_count: rom_banks,
            ram_bank_count: ram_banks,
        }
    }

    fn get_current_rom_bank(&self) -> u16 {
        let mut bank = self.state.rom_bank;
        if self.state.mbc1_mode {
            bank |= (self.state.ram_bank as u16) << 5;
        }
        bank % self.rom_bank_count
    }
}

impl MBCController for MBC1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // ROM Bank 0 (固定)
                if self.state.mbc1_mode {
                    let bank = ((self.state.ram_bank as u16) << 5) & (self.rom_bank_count - 1);
                    self.rom_data[addr as usize + (bank as usize * 0x4000)]
                } else {
                    self.rom_data[addr as usize]
                }
            }
            0x4000..=0x7FFF => {
                // ROM Bank 1-127
                let bank = self.get_current_rom_bank();
                let addr = addr as usize - 0x4000 + (bank as usize * 0x4000);
                if addr < self.rom_data.len() {
                    self.rom_data[addr]
                } else {
                    0xFF
                }
            }
            0xA000..=0xBFFF => {
                // External RAM
                if !self.state.ram_enabled {
                    return 0xFF;
                }
                let bank = if self.state.mbc1_mode {
                    self.state.ram_bank % self.ram_bank_count
                } else {
                    0
                };
                let addr = addr as usize - 0xA000 + (bank as usize * 0x2000);
                if addr < self.ram_data.len() {
                    self.ram_data[addr]
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM 啟用/禁用
                self.state.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number
                let mut bank = value & 0x1F;
                if bank == 0 {
                    bank = 1;
                }
                self.state.rom_bank = (self.state.rom_bank & 0x60) | bank as u16;
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number / Upper Bits of ROM Bank Number
                self.state.ram_bank = value & 0x03;
            }
            0x6000..=0x7FFF => {
                // Banking Mode Select
                self.state.mbc1_mode = (value & 0x01) != 0;
            }
            0xA000..=0xBFFF => {
                // External RAM
                if !self.state.ram_enabled {
                    return;
                }
                let bank = if self.state.mbc1_mode {
                    self.state.ram_bank % self.ram_bank_count
                } else {
                    0
                };
                let addr = addr as usize - 0xA000 + (bank as usize * 0x2000);
                if addr < self.ram_data.len() {
                    self.ram_data[addr] = value;
                }
            }
            _ => {}
        }
    }
}
