use super::MBCController;

#[derive(Debug)]
pub struct MBC1 {
    ram_enabled: bool,
    rom_bank: usize,
    ram_bank: usize,
    mode: bool, // false: ROM mode, true: RAM mode
}

impl MBC1 {
    pub fn new() -> Self {
        MBC1 {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            mode: false,
        }
    }
}

impl MBCController for MBC1 {
    fn read(&self, _addr: u16) -> u8 {
        0 // 簡化實現
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => {
                let bank = value & 0x1F;
                self.rom_bank = if bank == 0 { 1 } else { bank as usize };
            }
            0x4000..=0x5FFF => {
                self.ram_bank = (value & 0x03) as usize;
            }
            0x6000..=0x7FFF => {
                self.mode = value & 0x01 != 0;
            }
            _ => {}
        }
    }

    fn translate_rom_address(&self, addr: u16) -> u32 {
        match addr {
            0x0000..=0x3FFF => addr as u32,
            0x4000..=0x7FFF => {
                let bank = self.rom_bank;
                ((bank * 0x4000) + (addr as usize - 0x4000)) as u32
            }
            _ => addr as u32,
        }
    }

    fn translate_ram_address(&self, addr: u16) -> u16 {
        if !self.ram_enabled {
            return addr;
        }
        
        if self.mode {
            ((self.ram_bank * 0x2000) + (addr as usize)) as u16
        } else {
            addr
        }
    }

    fn current_rom_bank(&self) -> u8 {
        self.rom_bank as u8
    }
}
