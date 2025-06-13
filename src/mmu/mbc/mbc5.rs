use super::{MBCController, types::*};

pub struct MBC5 {
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    state: MBCState,
    rom_bank_count: u16,
    ram_bank_count: u8,
}

impl MBC5 {
    pub fn new(rom_data: Vec<u8>, ram_size: usize, rom_banks: u16, ram_banks: u8) -> Self {
        Self {
            rom_data,
            ram_data: vec![0; ram_size],
            state: MBCState::new(MBCType::MBC5),
            rom_bank_count: rom_banks,
            ram_bank_count: ram_banks,
        }
    }
}

impl MBCController for MBC5 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // ROM Bank 0 (固定)
                self.rom_data[addr as usize]
            }
            0x4000..=0x7FFF => {
                // ROM Bank 0-511
                let bank = self.state.rom_bank as usize;
                let addr = addr as usize - 0x4000 + (bank * 0x4000);
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
                let bank = self.state.ram_bank % self.ram_bank_count;
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
            0x2000..=0x2FFF => {
                // ROM Bank Number (低 8 位)
                self.state.rom_bank = (self.state.rom_bank & 0x100) | value as u16;
            }
            0x3000..=0x3FFF => {
                // ROM Bank Number (第 9 位)
                self.state.rom_bank = (self.state.rom_bank & 0xFF) | ((value as u16 & 0x01) << 8);
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number
                self.state.ram_bank = value & 0x0F;
            }
            0xA000..=0xBFFF => {
                // External RAM
                if !self.state.ram_enabled {
                    return;
                }
                let bank = self.state.ram_bank % self.ram_bank_count;
                let addr = addr as usize - 0xA000 + (bank as usize * 0x2000);
                if addr < self.ram_data.len() {
                    self.ram_data[addr] = value;
                }
            }
            _ => {}
        }
    }
}
