use super::{MBCController, types::*};

pub struct MBC3 {
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    state: MBCState,
    rom_bank_count: u16,
    ram_bank_count: u8,
    rtc_timestamp: u64,
}

impl MBC3 {
    pub fn new(rom_data: Vec<u8>, ram_size: usize, rom_banks: u16, ram_banks: u8) -> Self {
        Self {
            rom_data,
            ram_data: vec![0; ram_size],
            state: MBCState::new(MBCType::MBC3),
            rom_bank_count: rom_banks,
            ram_bank_count: ram_banks,
            rtc_timestamp: 0,
        }
    }

    fn update_rtc(&mut self) {
        // TODO: 實現實時時鐘更新
        // 這需要系統時間和遊戲時間的映射
    }
}

impl MBCController for MBC3 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // ROM Bank 0 (固定)
                self.rom_data[addr as usize]
            }
            0x4000..=0x7FFF => {
                // ROM Bank 1-127
                let bank = self.state.rom_bank as usize;
                let addr = addr as usize - 0x4000 + (bank * 0x4000);
                if addr < self.rom_data.len() {
                    self.rom_data[addr]
                } else {
                    0xFF
                }
            }
            0xA000..=0xBFFF => {
                if !self.state.ram_enabled {
                    return 0xFF;
                }
                if self.state.rtc_enabled {
                    // RTC 寄存器
                    let rtc_reg = self.state.ram_bank & 0x07;
                    if rtc_reg <= 4 {
                        self.state.rtc_registers[rtc_reg as usize]
                    } else {
                        0xFF
                    }
                } else {
                    // 正常 RAM 訪問
                    let bank = self.state.ram_bank % self.ram_bank_count;
                    let addr = addr as usize - 0xA000 + (bank as usize * 0x2000);
                    if addr < self.ram_data.len() {
                        self.ram_data[addr]
                    } else {
                        0xFF
                    }
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM 和 RTC 啟用/禁用
                self.state.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM Bank Number
                let mut bank = value & 0x7F;
                if bank == 0 {
                    bank = 1;
                }
                self.state.rom_bank = bank as u16;
            }
            0x4000..=0x5FFF => {
                // RAM Bank Number 或 RTC 寄存器選擇
                self.state.ram_bank = value;
                self.state.rtc_enabled = value >= 0x08 && value <= 0x0C;
            }
            0x6000..=0x7FFF => {
                // 鎖存 RTC 數據
                if value == 0x00 || value == 0x01 {
                    self.state.rtc_latched = value == 0x01;
                    if !self.state.rtc_latched {
                        self.update_rtc();
                    }
                }
            }
            0xA000..=0xBFFF => {
                if !self.state.ram_enabled {
                    return;
                }
                if self.state.rtc_enabled {
                    // 寫入 RTC 寄存器
                    let rtc_reg = self.state.ram_bank & 0x07;
                    if rtc_reg <= 4 {
                        self.state.rtc_registers[rtc_reg as usize] = value;
                    }
                } else {
                    // 正常 RAM 寫入
                    let bank = self.state.ram_bank % self.ram_bank_count;
                    let addr = addr as usize - 0xA000 + (bank as usize * 0x2000);
                    if addr < self.ram_data.len() {
                        self.ram_data[addr] = value;
                    }
                }
            }
            _ => {}
        }
    }
}
