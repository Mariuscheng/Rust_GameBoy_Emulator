// Game Boy Memory Bank Controller (MBC) 實現
// 負責處理不同類型的卡帶記憶體控制器

/// MBC 類型枚舉
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MBCType {
    None,        // 無 MBC (ROM Only)
    MBC1,        // MBC1 - 最常見的控制器
    MBC2,        // MBC2 - 內建 RAM
    MBC3,        // MBC3 - 支援 RTC
    MBC5,        // MBC5 - 最先進的控制器
    Unknown(u8), // 未知類型
}

impl MBCType {
    /// 從卡帶類型代碼創建 MBC 類型
    pub fn from_cartridge_type(cartridge_type: u8) -> Self {
        match cartridge_type {
            0x00 => MBCType::None,
            0x01..=0x03 => MBCType::MBC1,
            0x05..=0x06 => MBCType::MBC2,
            0x0F..=0x13 => MBCType::MBC3,
            0x19..=0x1E => MBCType::MBC5,
            _ => MBCType::Unknown(cartridge_type),
        }
    }

    /// 取得 MBC 類型的描述
    pub fn description(&self) -> &'static str {
        match self {
            MBCType::None => "ROM Only",
            MBCType::MBC1 => "MBC1",
            MBCType::MBC2 => "MBC2 + Battery",
            MBCType::MBC3 => "MBC3 + Timer + Battery",
            MBCType::MBC5 => "MBC5",
            MBCType::Unknown(_code) => "Unknown MBC",
        }
    }
}

/// ROM 大小計算
pub fn get_rom_size_bytes(rom_size_code: u8) -> usize {
    match rom_size_code {
        0x00 => 32 * 1024,   // 32KB
        0x01 => 64 * 1024,   // 64KB
        0x02 => 128 * 1024,  // 128KB
        0x03 => 256 * 1024,  // 256KB
        0x04 => 512 * 1024,  // 512KB
        0x05 => 1024 * 1024, // 1MB
        0x06 => 2048 * 1024, // 2MB
        0x07 => 4096 * 1024, // 4MB
        0x08 => 8192 * 1024, // 8MB
        _ => 32 * 1024,      // 默認 32KB
    }
}

/// RAM 大小計算
pub fn get_ram_size_bytes(ram_size_code: u8) -> usize {
    match ram_size_code {
        0x00 => 0,          // 無 RAM
        0x01 => 2 * 1024,   // 2KB
        0x02 => 8 * 1024,   // 8KB
        0x03 => 32 * 1024,  // 32KB (4 banks of 8KB)
        0x04 => 128 * 1024, // 128KB (16 banks of 8KB)
        0x05 => 64 * 1024,  // 64KB (8 banks of 8KB)
        _ => 0,             // 默認無 RAM
    }
}

/// MBC 控制器狀態
#[derive(Debug, Clone)]
pub struct MBCState {
    pub mbc_type: MBCType,
    pub rom_bank: u16,          // 當前 ROM bank
    pub ram_bank: u8,           // 當前 RAM bank
    pub ram_enabled: bool,      // RAM 是否啟用
    pub mbc1_mode: bool,        // MBC1 模式 (false=ROM模式, true=RAM模式)
    pub rtc_enabled: bool,      // MBC3 RTC 是否啟用
    pub rtc_latched: bool,      // MBC3 RTC 鎖存狀態
    pub rtc_registers: [u8; 5], // MBC3 RTC 暫存器 (S, M, H, DL, DH)
    pub battery_backed: bool,   // 是否有電池備份
}

impl MBCState {
    /// 創建新的 MBC 狀態
    pub fn new(mbc_type: MBCType) -> Self {
        Self {
            mbc_type,
            rom_bank: if mbc_type == MBCType::None { 0 } else { 1 },
            ram_bank: 0,
            ram_enabled: false,
            mbc1_mode: false,
            rtc_enabled: false,
            rtc_latched: false,
            rtc_registers: [0; 5],
            battery_backed: false,
        }
    }

    /// 取得有效的 ROM bank 號
    pub fn get_effective_rom_bank(&self, total_banks: u16) -> u16 {
        match self.mbc_type {
            MBCType::None => 0,
            MBCType::MBC1 => {
                let mut bank = self.rom_bank;
                if bank == 0 {
                    bank = 1;
                } // MBC1 不能選擇 bank 0
                bank % total_banks
            }
            MBCType::MBC2 => {
                let mut bank = self.rom_bank & 0x0F;
                if bank == 0 {
                    bank = 1;
                }
                bank % total_banks
            }
            MBCType::MBC3 | MBCType::MBC5 => {
                let mut bank = self.rom_bank;
                if bank == 0 {
                    bank = 1;
                }
                bank % total_banks
            }
            MBCType::Unknown(_) => 1,
        }
    }

    /// 取得有效的 RAM bank 號
    pub fn get_effective_ram_bank(&self, total_banks: u8) -> u8 {
        if total_banks == 0 {
            return 0;
        }
        self.ram_bank % total_banks
    }
}

/// MBC 控制器實現
pub struct MBCController {
    pub state: MBCState,
    rom_data: Vec<u8>,
    ram_data: Vec<u8>,
    rom_bank_count: u16,
    ram_bank_count: u8,
    rtc_timestamp: u64, // 用於 RTC 計算
}

impl MBCController {
    /// 創建新的 MBC 控制器
    pub fn new(rom_data: Vec<u8>) -> Self {
        let mbc_type = if rom_data.len() >= 0x148 {
            MBCType::from_cartridge_type(rom_data[0x147])
        } else {
            MBCType::None
        };

        let rom_size_code = if rom_data.len() >= 0x149 {
            rom_data[0x148]
        } else {
            0
        };
        let ram_size_code = if rom_data.len() >= 0x14A {
            rom_data[0x149]
        } else {
            0
        };

        let rom_bank_count = (get_rom_size_bytes(rom_size_code) / 0x4000) as u16;
        let ram_size = get_ram_size_bytes(ram_size_code);
        let ram_bank_count = if ram_size > 0 {
            (ram_size / 0x2000) as u8
        } else {
            0
        };

        println!("🎮 MBC 控制器初始化:");
        println!("  - 類型: {:?} ({})", mbc_type, mbc_type.description());
        println!(
            "  - ROM 大小: {} KB ({} banks)",
            rom_data.len() / 1024,
            rom_bank_count
        );
        println!(
            "  - RAM 大小: {} KB ({} banks)",
            ram_size / 1024,
            ram_bank_count
        );

        Self {
            state: MBCState::new(mbc_type),
            rom_data,
            ram_data: vec![0; ram_size],
            rom_bank_count,
            ram_bank_count,
            rtc_timestamp: 0,
        }
    }

    /// 讀取 ROM 數據
    pub fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // Bank 0 - 始終可見
                self.rom_data.get(addr as usize).copied().unwrap_or(0xFF)
            }
            0x4000..=0x7FFF => {
                // Banked ROM
                let bank = self.state.get_effective_rom_bank(self.rom_bank_count);
                let offset = (bank as usize) * 0x4000 + ((addr as usize) & 0x3FFF);
                self.rom_data.get(offset).copied().unwrap_or(0xFF)
            }
            _ => 0xFF,
        }
    }

    /// 讀取 RAM 數據
    pub fn read_ram(&self, addr: u16) -> u8 {
        if !self.state.ram_enabled || self.ram_bank_count == 0 {
            return 0xFF;
        }

        match self.state.mbc_type {
            MBCType::MBC2 => {
                // MBC2 內建 512 x 4 位 RAM
                if addr >= 0xA000 && addr <= 0xA1FF {
                    let index = (addr - 0xA000) as usize;
                    self.ram_data.get(index).copied().unwrap_or(0xFF) | 0xF0
                } else {
                    0xFF
                }
            }
            MBCType::MBC3 => {
                if addr >= 0xA000 && addr <= 0xBFFF {
                    if self.state.ram_bank <= 0x03 {
                        // RAM banks
                        let bank = self.state.get_effective_ram_bank(self.ram_bank_count);
                        let offset = (bank as usize) * 0x2000 + ((addr as usize) & 0x1FFF);
                        self.ram_data.get(offset).copied().unwrap_or(0xFF)
                    } else if self.state.ram_bank >= 0x08 && self.state.ram_bank <= 0x0C {
                        // RTC registers
                        let rtc_index = (self.state.ram_bank - 0x08) as usize;
                        self.state
                            .rtc_registers
                            .get(rtc_index)
                            .copied()
                            .unwrap_or(0xFF)
                    } else {
                        0xFF
                    }
                } else {
                    0xFF
                }
            }
            _ => {
                // 標準 RAM 訪問
                if addr >= 0xA000 && addr <= 0xBFFF {
                    let bank = self.state.get_effective_ram_bank(self.ram_bank_count);
                    let offset = (bank as usize) * 0x2000 + ((addr as usize) & 0x1FFF);
                    self.ram_data.get(offset).copied().unwrap_or(0xFF)
                } else {
                    0xFF
                }
            }
        }
    }

    /// 寫入 RAM 數據
    pub fn write_ram(&mut self, addr: u16, value: u8) {
        if !self.state.ram_enabled || self.ram_bank_count == 0 {
            return;
        }

        match self.state.mbc_type {
            MBCType::MBC2 => {
                // MBC2 內建 512 x 4 位 RAM
                if addr >= 0xA000 && addr <= 0xA1FF {
                    let index = (addr - 0xA000) as usize;
                    if index < self.ram_data.len() {
                        self.ram_data[index] = value & 0x0F;
                    }
                }
            }
            MBCType::MBC3 => {
                if addr >= 0xA000 && addr <= 0xBFFF {
                    if self.state.ram_bank <= 0x03 {
                        // RAM banks
                        let bank = self.state.get_effective_ram_bank(self.ram_bank_count);
                        let offset = (bank as usize) * 0x2000 + ((addr as usize) & 0x1FFF);
                        if offset < self.ram_data.len() {
                            self.ram_data[offset] = value;
                        }
                    } else if self.state.ram_bank >= 0x08 && self.state.ram_bank <= 0x0C {
                        // RTC registers
                        let rtc_index = (self.state.ram_bank - 0x08) as usize;
                        if rtc_index < self.state.rtc_registers.len() {
                            self.state.rtc_registers[rtc_index] = value;
                        }
                    }
                }
            }
            _ => {
                // 標準 RAM 寫入
                if addr >= 0xA000 && addr <= 0xBFFF {
                    let bank = self.state.get_effective_ram_bank(self.ram_bank_count);
                    let offset = (bank as usize) * 0x2000 + ((addr as usize) & 0x1FFF);
                    if offset < self.ram_data.len() {
                        self.ram_data[offset] = value;
                    }
                }
            }
        }
    }

    /// 處理 MBC 控制寫入
    pub fn write_control(&mut self, addr: u16, value: u8) {
        match self.state.mbc_type {
            MBCType::None => {}
            MBCType::MBC1 => self.handle_mbc1_control(addr, value),
            MBCType::MBC2 => self.handle_mbc2_control(addr, value),
            MBCType::MBC3 => self.handle_mbc3_control(addr, value),
            MBCType::MBC5 => self.handle_mbc5_control(addr, value),
            MBCType::Unknown(_) => {}
        }
    }

    /// MBC1 控制處理
    fn handle_mbc1_control(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM 啟用/禁用
                self.state.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM bank 選擇 (低 5 位)
                let bank = value & 0x1F;
                let bank = if bank == 0 { 1 } else { bank };
                self.state.rom_bank = (self.state.rom_bank & 0x60) | (bank as u16);
            }
            0x4000..=0x5FFF => {
                // ROM/RAM bank 選擇
                if self.state.mbc1_mode {
                    // RAM 模式
                    self.state.ram_bank = value & 0x03;
                } else {
                    // ROM 模式
                    self.state.rom_bank =
                        (self.state.rom_bank & 0x1F) | (((value & 0x03) as u16) << 5);
                }
            }
            0x6000..=0x7FFF => {
                // 模式選擇
                self.state.mbc1_mode = (value & 0x01) != 0;
            }
            _ => {}
        }
    }

    /// MBC2 控制處理
    fn handle_mbc2_control(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                if (addr & 0x0100) == 0 {
                    self.state.ram_enabled = (value & 0x0F) == 0x0A;
                }
            }
            0x2000..=0x3FFF => {
                if (addr & 0x0100) != 0 {
                    let mut bank = value & 0x0F;
                    if bank == 0 {
                        bank = 1;
                    }
                    self.state.rom_bank = bank as u16;
                }
            }
            _ => {}
        }
    }

    /// MBC3 控制處理
    fn handle_mbc3_control(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM/RTC 啟用
                self.state.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM bank 選擇
                let mut bank = value & 0x7F;
                if bank == 0 {
                    bank = 1;
                }
                self.state.rom_bank = bank as u16;
            }
            0x4000..=0x5FFF => {
                // RAM/RTC bank 選擇
                self.state.ram_bank = value;
                if value >= 0x08 && value <= 0x0C {
                    self.state.rtc_enabled = true;
                }
            }
            0x6000..=0x7FFF => {
                // RTC 鎖存
                if value == 0x00 {
                    self.state.rtc_latched = false;
                } else if value == 0x01 && !self.state.rtc_latched {
                    self.latch_rtc();
                    self.state.rtc_latched = true;
                }
            }
            _ => {}
        }
    }

    /// MBC5 控制處理
    fn handle_mbc5_control(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM 啟用
                self.state.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x2FFF => {
                // ROM bank 低 8 位
                self.state.rom_bank = (self.state.rom_bank & 0x100) | (value as u16);
            }
            0x3000..=0x3FFF => {
                // ROM bank 第 9 位
                self.state.rom_bank = (self.state.rom_bank & 0xFF) | (((value & 0x01) as u16) << 8);
            }
            0x4000..=0x5FFF => {
                // RAM bank 選擇
                self.state.ram_bank = value & 0x0F;
            }
            _ => {}
        }
    }

    /// 鎖存 RTC 數據
    fn latch_rtc(&mut self) {
        // 簡化的 RTC 實現
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let elapsed = current_time - self.rtc_timestamp;

        let seconds = (elapsed % 60) as u8;
        let minutes = ((elapsed / 60) % 60) as u8;
        let hours = ((elapsed / 3600) % 24) as u8;
        let days = (elapsed / 86400) as u16;

        self.state.rtc_registers[0] = seconds;
        self.state.rtc_registers[1] = minutes;
        self.state.rtc_registers[2] = hours;
        self.state.rtc_registers[3] = (days & 0xFF) as u8;
        self.state.rtc_registers[4] = ((days >> 8) & 0x01) as u8;
    }

    /// 取得狀態報告
    pub fn get_status_report(&self) -> String {
        format!(
            "MBC 狀態報告:\n\
             類型: {:?} ({})\n\
             ROM Bank: {} / {}\n\
             RAM Bank: {} / {} (啟用: {})\n\
             模式: {}\n\
             RTC 啟用: {}",
            self.state.mbc_type,
            self.state.mbc_type.description(),
            self.state.rom_bank,
            self.rom_bank_count,
            self.state.ram_bank,
            self.ram_bank_count,
            self.state.ram_enabled,
            if self.state.mbc1_mode { "RAM" } else { "ROM" },
            self.state.rtc_enabled
        )
    }

    /// 載入保存數據 (電池備份)
    pub fn load_save_data(&mut self, data: Vec<u8>) {
        if data.len() == self.ram_data.len() {
            self.ram_data = data;
            println!("✅ 載入存檔數據: {} bytes", self.ram_data.len());
        } else {
            println!(
                "⚠️ 存檔數據大小不匹配: 預期 {}, 實際 {}",
                self.ram_data.len(),
                data.len()
            );
        }
    }

    /// 保存數據 (電池備份)
    pub fn get_save_data(&self) -> Vec<u8> {
        self.ram_data.clone()
    }

    /// 重置 MBC 狀態
    pub fn reset(&mut self) {
        self.state = MBCState::new(self.state.mbc_type);
        self.rtc_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbc_type_detection() {
        assert_eq!(MBCType::from_cartridge_type(0x00), MBCType::None);
        assert_eq!(MBCType::from_cartridge_type(0x01), MBCType::MBC1);
        assert_eq!(MBCType::from_cartridge_type(0x05), MBCType::MBC2);
        assert_eq!(MBCType::from_cartridge_type(0x0F), MBCType::MBC3);
        assert_eq!(MBCType::from_cartridge_type(0x19), MBCType::MBC5);
    }

    #[test]
    fn test_rom_size_calculation() {
        assert_eq!(get_rom_size_bytes(0x00), 32 * 1024);
        assert_eq!(get_rom_size_bytes(0x01), 64 * 1024);
        assert_eq!(get_rom_size_bytes(0x05), 1024 * 1024);
    }

    #[test]
    fn test_ram_size_calculation() {
        assert_eq!(get_ram_size_bytes(0x00), 0);
        assert_eq!(get_ram_size_bytes(0x02), 8 * 1024);
        assert_eq!(get_ram_size_bytes(0x03), 32 * 1024);
    }

    #[test]
    fn test_mbc1_rom_banking() {
        let rom_data = vec![0; 128 * 1024]; // 128KB ROM
        let mut mbc = MBCController::new(rom_data);
        mbc.state.mbc_type = MBCType::MBC1;

        // 測試 ROM bank 切換
        mbc.handle_mbc1_control(0x2000, 0x02);
        assert_eq!(mbc.state.rom_bank, 2);

        // 測試 bank 0 自動變為 bank 1
        mbc.handle_mbc1_control(0x2000, 0x00);
        assert_eq!(mbc.state.rom_bank, 1);
    }
}
