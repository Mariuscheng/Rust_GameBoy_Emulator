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
        log::info!("初始化 MBC1 控制器");
        MBC1 {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            mode: false,
        }
    }

    fn log_state(&self) {
        log::debug!(
            "MBC1 狀態: ROM Bank={}, RAM Bank={}, Mode={}, RAM Enabled={}",
            self.rom_bank,
            self.ram_bank,
            if self.mode { "RAM" } else { "ROM" },
            self.ram_enabled
        );
    }
}

impl MBCController for MBC1 {
    fn read(&self, addr: u16) -> u8 {
        log::trace!("MBC1 讀取: 位址={:04X}", addr);
        0xFF // 實際資料讀取由 MMU 處理
    }

    fn write(&mut self, addr: u16, value: u8) {
        log::trace!("MBC1 寫入: 位址={:04X}, 值={:02X}", addr, value);

        match addr {
            0x0000..=0x1FFF => {
                let old_state = self.ram_enabled;
                self.ram_enabled = value & 0x0F == 0x0A;
                if old_state != self.ram_enabled {
                    log::info!(
                        "RAM {} (值={:02X})",
                        if self.ram_enabled { "啟用" } else { "禁用" },
                        value
                    );
                }
            }
            0x2000..=0x3FFF => {
                let bank = value & 0x1F;
                let new_bank = if bank == 0 { 1 } else { bank as usize };
                if self.rom_bank != new_bank {
                    log::info!(
                        "切換 ROM Bank: {} -> {} (值={:02X})",
                        self.rom_bank,
                        new_bank,
                        value
                    );
                }
                self.rom_bank = new_bank;
            }
            0x4000..=0x5FFF => {
                let new_bank = (value & 0x03) as usize;
                if self.ram_bank != new_bank {
                    log::info!(
                        "切換 RAM Bank: {} -> {} (值={:02X})",
                        self.ram_bank,
                        new_bank,
                        value
                    );
                }
                self.ram_bank = new_bank;
            }
            0x6000..=0x7FFF => {
                let new_mode = value & 0x01 != 0;
                if self.mode != new_mode {
                    log::info!(
                        "切換 MBC1 模式: {} -> {} (值={:02X})",
                        if self.mode { "RAM" } else { "ROM" },
                        if new_mode { "RAM" } else { "ROM" },
                        value
                    );
                }
                self.mode = new_mode;
            }
            _ => {
                log::warn!("MBC1: 嘗試寫入無效位址 {:04X} = {:02X}", addr, value);
            }
        }

        self.log_state();
    }

    fn translate_rom_address(&self, addr: u16) -> u32 {
        match addr {
            0x0000..=0x3FFF => {
                log::trace!("MBC1 訪問 ROM Bank 0: {:04X}", addr);
                addr as u32
            }
            0x4000..=0x7FFF => {
                let bank = self.rom_bank;
                let physical_addr = ((bank * 0x4000) + (addr as usize - 0x4000)) as u32;
                log::trace!(
                    "MBC1 訪問 ROM Bank {}: 邏輯位址={:04X} -> 物理位址={:06X}",
                    bank,
                    addr,
                    physical_addr
                );
                physical_addr
            }
            _ => {
                log::warn!("MBC1: 嘗試訪問無效的 ROM 位址 {:04X}", addr);
                addr as u32
            }
        }
    }

    fn translate_ram_address(&self, addr: u16) -> u16 {
        if !self.ram_enabled {
            log::warn!("MBC1: 嘗試訪問已禁用的 RAM {:04X}", addr);
            return addr;
        }

        let physical_addr = if self.mode {
            let addr = ((self.ram_bank * 0x2000) + (addr as usize)) as u16;
            log::trace!(
                "MBC1 訪問 RAM Bank {}: 邏輯位址={:04X} -> 物理位址={:04X}",
                self.ram_bank,
                addr,
                addr
            );
            addr
        } else {
            log::trace!("MBC1 訪問 RAM Bank 0: {:04X}", addr);
            addr
        };
        physical_addr
    }

    fn current_rom_bank(&self) -> u8 {
        self.rom_bank as u8
    }
}
