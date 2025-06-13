use super::{MBCController, types::*};

pub struct MBC2 {
    rom_data: Vec<u8>,
    ram_data: [u8; 512], // MBC2 有 512x4 bits 的內置 RAM
    state: MBCState,
    rom_bank_count: u16,
}

impl MBC2 {
    pub fn new(rom_data: Vec<u8>, rom_banks: u16) -> Self {
        Self {
            rom_data,
            ram_data: [0; 512],
            state: MBCState::new(MBCType::MBC2),
            rom_bank_count: rom_banks,
        }
    }
}

impl MBCController for MBC2 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // ROM Bank 0 (固定)
                self.rom_data[addr as usize]
            }
            0x4000..=0x7FFF => {
                // ROM Bank 1-15
                let bank = self.state.rom_bank as usize;
                let addr = addr as usize - 0x4000 + (bank * 0x4000);
                if addr < self.rom_data.len() {
                    self.rom_data[addr]
                } else {
                    0xFF
                }
            }
            0xA000..=0xA1FF => {
                // 內置 RAM (512x4 bits)
                if !self.state.ram_enabled {
                    return 0xFF;
                }
                self.ram_data[addr as usize - 0xA000] & 0x0F
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x3FFF => {
                // RAM 啟用/禁用 和 ROM Bank 選擇
                if (addr & 0x0100) == 0 {
                    // RAM 啟用/禁用 (bit 8 = 0)
                    self.state.ram_enabled = (value & 0x0F) == 0x0A;
                } else {
                    // ROM Bank 選擇 (bit 8 = 1)
                    let mut bank = value & 0x0F;
                    if bank == 0 {
                        bank = 1;
                    }
                    self.state.rom_bank = bank as u16;
                }
            }
            0xA000..=0xA1FF => {
                // 內置 RAM (只有低 4 位有效)
                if !self.state.ram_enabled {
                    return;
                }
                self.ram_data[addr as usize - 0xA000] = value & 0x0F;
            }
            _ => {}
        }
    }
}
