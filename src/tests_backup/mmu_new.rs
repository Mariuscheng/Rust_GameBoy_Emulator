/*
================================================================================
Game Boy 模擬器 - MMU 舊版本 (備份)
================================================================================
這是之前的 MMU 實現版本，保存在這裡作為參考和調試使用。
此文件包含了舊的ROM驗證邏輯，可能導致了白螢幕問題。

問題點：
- 舊版本要求ROM最小為0x150字節
- 沒有對測試ROM的特殊處理
- 可能被系統誤用導致編譯問題

日期: 2025年6月10日
================================================================================
*/

use crate::apu::APU;
use crate::joypad::Joypad;
use crate::timer::Timer;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomState {
    Uninitialized, // ROM從未載入
    Empty,         // ROM數據為空
    Invalid,       // ROM數據無效
    Valid,         // ROM數據有效
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MBCType {
    None,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
}

pub struct MBC {
    pub mbc_type: MBCType,
    pub rom_bank: u16,
    pub ram_bank: u8,
    pub ram_enabled: bool,
    pub banking_mode: u8,
}

impl MBC {
    pub fn new() -> Self {
        Self {
            mbc_type: MBCType::None,
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            banking_mode: 0,
        }
    }
}

pub struct MMU {
    pub memory: [u8; 0x10000],
    pub rom: Vec<u8>,
    pub fallback_rom: Vec<u8>,
    pub mbc: MBC,
    pub rom_state: RomState,

    // 組件引用
    pub apu: Rc<RefCell<APU>>,
    pub timer: Rc<RefCell<Timer>>,

    // 狀態和調試
    debug_enabled: bool,
    memory_access_count: u64,
    vram_write_count: u64,
    rom_access_count: u64,
}

impl MMU {
    pub fn new() -> Self {
        let fallback = Self::create_fallback_rom();
        println!("創建fallback ROM ({} bytes)", fallback.len());

        Self {
            memory: [0; 0x10000],
            rom: fallback.clone(),
            fallback_rom: fallback,
            mbc: MBC::new(),
            rom_state: RomState::Uninitialized,
            apu: Rc::new(RefCell::new(APU::new())),
            timer: Rc::new(RefCell::new(Timer::new())),
            debug_enabled: true,
            memory_access_count: 0,
            vram_write_count: 0,
            rom_access_count: 0,
        }
    }

    fn create_fallback_rom() -> Vec<u8> {
        let mut fallback = vec![0; 0x8000];

        // 設置基本的 ROM header
        fallback[0x0143] = 0x00; // 非 CGB 模式
        fallback[0x0147] = 0x00; // ROM ONLY
        fallback[0x0148] = 0x00; // 32KB ROM
        fallback[0x0149] = 0x00; // 無 RAM

        // 添加 Nintendo logo 校驗和
        let nintendo_logo = [
            0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C,
            0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6,
            0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC,
            0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
        ];
        for (i, &byte) in nintendo_logo.iter().enumerate() {
            fallback[0x0104 + i] = byte;
        }

        // 計算並設置 header 校驗和
        let mut checksum: u8 = 0;
        for i in 0x0134..0x014D {
            checksum = checksum.wrapping_sub(fallback[i]).wrapping_sub(1);
        }
        fallback[0x014D] = checksum;

        fallback
    }

    // *** 舊版本的 load_rom 方法 - 這是導致問題的代碼 ***
    pub fn load_rom(&mut self, rom_data: Vec<u8>) {
        println!("正在載入ROM... (大小: {} bytes)", rom_data.len());

        self.rom_state = RomState::Empty;

        if rom_data.is_empty() {
            println!("警告：ROM數據為空，將使用fallback ROM");
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Empty;
            self.mbc.mbc_type = MBCType::None;
            return;
        }

        // *** 問題所在：舊版本沒有對測試ROM的特殊處理 ***
        if rom_data.len() < 0x150 {
            println!("警告：ROM太小 (< 0x150 bytes)，將使用fallback ROM");
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Invalid;
            self.mbc.mbc_type = MBCType::None;
            return;
        }

        self.rom = rom_data;

        if self.validate_and_setup_rom() {
            self.rom_state = RomState::Valid;
            println!("ROM載入成功，狀態: {:?}", self.rom_state);
        } else {
            println!("警告：ROM驗證失敗，將使用fallback ROM");
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Invalid;
            self.mbc.mbc_type = MBCType::None;
        }
    }

    fn validate_and_setup_rom(&mut self) -> bool {
        if self.rom.len() > 0x147 {
            let cartridge_type = self.rom[0x147];
            self.mbc.mbc_type = match cartridge_type {
                0x00 => MBCType::None,
                0x01..=0x03 => MBCType::MBC1,
                0x05..=0x06 => MBCType::MBC2,
                0x0F..=0x13 => MBCType::MBC3,
                0x19..=0x1E => MBCType::MBC5,
                _ => {
                    println!("不支援的 MBC 類型: 0x{:02X}", cartridge_type);
                    return false;
                }
            };

            // 驗證 Nintendo logo
            let nintendo_logo = [
                0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C,
                0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6,
                0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC,
                0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
            ];

            for (i, &expected) in nintendo_logo.iter().enumerate() {
                if self.rom[0x0104 + i] != expected {
                    println!("Nintendo logo 驗證失敗於位置: 0x{:04X}", 0x0104 + i);
                    return false;
                }
            }

            // 驗證 header checksum
            let mut checksum: u8 = 0;
            for i in 0x0134..0x014D {
                checksum = checksum.wrapping_sub(self.rom[i]).wrapping_sub(1);
            }

            if checksum != self.rom[0x014D] {
                println!(
                    "Header checksum 驗證失敗: 計算值={:02X}, ROM值={:02X}",
                    checksum, self.rom[0x014D]
                );
                return false;
            }

            return true;
        }

        false
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        self.memory_access_count += 1;

        match address {
            0x0000..=0x7FFF => {
                // ROM 區域
                self.rom_access_count += 1;
                let rom_address = self.calculate_rom_address(address);
                if rom_address < self.rom.len() {
                    self.rom[rom_address]
                } else {
                    0xFF
                }
            }
            0x8000..=0x9FFF => {
                // VRAM 區域
                self.memory[address as usize]
            }
            0xA000..=0xBFFF => {
                // 外部 RAM 區域
                if self.mbc.ram_enabled {
                    self.memory[address as usize]
                } else {
                    0xFF
                }
            }
            0xC000..=0xFDFF => {
                // 工作 RAM 和 Echo RAM
                self.memory[address as usize]
            }
            0xFE00..=0xFE9F => {
                // OAM 區域
                self.memory[address as usize]
            }
            0xFEA0..=0xFEFF => {
                // 未使用區域
                0xFF
            }
            0xFF00..=0xFFFF => {
                // I/O 寄存器和高速 RAM
                match address {
                    0xFF00 => {
                        // 手柄寄存器 - 應該從 joypad 模組讀取
                        0xFF // 暫時返回默認值
                    }
                    0xFF40..=0xFF4B => {
                        // PPU 寄存器
                        self.memory[address as usize]
                    }
                    _ => self.memory[address as usize],
                }
            }
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.memory_access_count += 1;

        match address {
            0x0000..=0x7FFF => {
                // ROM 區域 - MBC 控制
                self.handle_mbc_write(address, value);
            }
            0x8000..=0x9FFF => {
                // VRAM 區域
                self.memory[address as usize] = value;
                self.vram_write_count += 1;
            }
            0xA000..=0xBFFF => {
                // 外部 RAM 區域
                if self.mbc.ram_enabled {
                    self.memory[address as usize] = value;
                }
            }
            0xC000..=0xFDFF => {
                // 工作 RAM 和 Echo RAM
                self.memory[address as usize] = value;
                // Echo RAM 同步
                if address >= 0xC000 && address <= 0xDDFF {
                    self.memory[(address + 0x2000) as usize] = value;
                } else if address >= 0xE000 && address <= 0xFDFF {
                    self.memory[(address - 0x2000) as usize] = value;
                }
            }
            0xFE00..=0xFE9F => {
                // OAM 區域
                self.memory[address as usize] = value;
            }
            0xFEA0..=0xFEFF => {
                // 未使用區域 - 忽略寫入
            }
            0xFF00..=0xFFFF => {
                // I/O 寄存器和高速 RAM
                self.memory[address as usize] = value;
            }
        }
    }

    fn calculate_rom_address(&self, address: u16) -> usize {
        match self.mbc.mbc_type {
            MBCType::None => address as usize,
            MBCType::MBC1 => {
                if address < 0x4000 {
                    address as usize
                } else {
                    ((self.mbc.rom_bank as usize) * 0x4000) + ((address as usize) - 0x4000)
                }
            }
            _ => address as usize, // 簡化實現
        }
    }

    fn handle_mbc_write(&mut self, address: u16, value: u8) {
        match self.mbc.mbc_type {
            MBCType::MBC1 => {
                match address {
                    0x0000..=0x1FFF => {
                        // RAM Enable
                        self.mbc.ram_enabled = (value & 0x0F) == 0x0A;
                    }
                    0x2000..=0x3FFF => {
                        // ROM Bank Number
                        let bank = (value & 0x1F) as u16;
                        self.mbc.rom_bank = if bank == 0 { 1 } else { bank };
                    }
                    0x4000..=0x5FFF => {
                        // RAM Bank Number / Upper Bits of ROM Bank Number
                        self.mbc.ram_bank = value & 0x03;
                    }
                    0x6000..=0x7FFF => {
                        // Banking Mode Select
                        self.mbc.banking_mode = value & 0x01;
                    }
                    _ => {}
                }
            }
            _ => {
                // 其他 MBC 類型的簡化處理
            }
        }
    }

    // 其他必要的方法...
    pub fn vram(&self) -> &[u8] {
        &self.memory[0x8000..0xA000]
    }

    pub fn oam(&self) -> &[u8] {
        &self.memory[0xFE00..0xFEA0]
    }

    pub fn step(&mut self) {
        // MMU 每步更新邏輯
    }

    pub fn step_apu(&mut self) {
        // APU 步進邏輯
    }

    pub fn set_joypad(&mut self, _state: u8) {
        // 手柄狀態設置
    }

    pub fn get_debug_info(&self) -> String {
        format!(
            "MMU 調試信息 (舊版本):\n\
            ROM 狀態: {:?}\n\
            MBC 類型: {:?}\n\
            ROM Bank: {}\n\
            RAM Bank: {}\n\
            內存訪問次數: {}\n\
            VRAM 寫入次數: {}\n\
            ROM 訪問次數: {}",
            self.rom_state,
            self.mbc.mbc_type,
            self.mbc.rom_bank,
            self.mbc.ram_bank,
            self.memory_access_count,
            self.vram_write_count,
            self.rom_access_count
        )
    }
}
