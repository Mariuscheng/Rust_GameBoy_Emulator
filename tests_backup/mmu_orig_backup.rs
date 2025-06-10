/*
================================================================================
Game Boy 模擬器 - MMU 改進版本 (2025年版)
================================================================================
這是基於當前 MMU 實現的改進版本，解決了以下問題：
1. ROM 載入邏輯優化
2. 更好的測試 ROM 支援
3. 增強的調試功能
4. 改進的錯誤處理
5. 更詳細的 VRAM 分析

主要改進：
- 支援最小 16 字節的測試 ROM
- 跳過小 ROM 的標準驗證
- 增強的 VRAM 寫入調試
- 詳細的 ROM 狀態追蹤
- 改進的記憶體安全檢查

版本: 2.0 (改進版)
日期: 2025年6月10日
基於: src/mmu.rs (當前實現)
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
    Empty,         // ROM載入但為空
    Invalid,       // ROM載入但格式無效
    Valid,         // ROM載入且有效
    TestRom,       // 特殊標記：測試ROM（小於標準大小但有效）
}

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

#[derive(Debug, Clone, Copy)]
pub struct RomInfo {
    pub size: usize,
    pub state: RomState,
    pub mbc_type: MBCType,
    pub is_test_rom: bool,
    pub has_nintendo_logo: bool,
    pub checksum_valid: bool,
}

impl RomInfo {
    pub fn new() -> Self {
        Self {
            size: 0,
            state: RomState::Uninitialized,
            mbc_type: MBCType::None,
            is_test_rom: false,
            has_nintendo_logo: false,
            checksum_valid: false,
        }
    }
}

pub struct MMU {
    memory: [u8; 0x10000],
    pub vram: Rc<RefCell<[u8; 0x2000]>>,
    pub oam: Rc<RefCell<[u8; 0xA0]>>,
    pub rom: Vec<u8>,
    pub rom_state: RomState,
    pub rom_info: RomInfo, // 新增：詳細的ROM信息
    fallback_rom: Vec<u8>,
    pub if_reg: u8,
    pub ie_reg: u8,
    joypad: Joypad,
    timer: Timer,
    apu: Rc<RefCell<APU>>,
    mbc: MBCController,

    // 新增：調試相關欄位
    pub debug_mode: bool,
    pub vram_write_count: u32,
    pub rom_read_count: u32,
}

impl MMU {
    pub fn new() -> Self {
        let vram = Rc::new(RefCell::new([0; 0x2000]));
        let oam = Rc::new(RefCell::new([0; 0xA0]));
        let apu = Rc::new(RefCell::new(APU::new()));

        let mut mmu = Self {
            memory: [0; 0x10000],
            vram,
            oam,
            rom: Vec::new(),
            rom_state: RomState::Uninitialized,
            rom_info: RomInfo::new(),
            fallback_rom: Vec::new(),
            if_reg: 0,
            ie_reg: 0,
            joypad: Joypad::new(),
            timer: Timer::new(),
            apu,
            mbc: MBCController::new(MBCType::None),
            debug_mode: true, // 默認開啟調試模式
            vram_write_count: 0,
            rom_read_count: 0,
        };

        // 創建功能更強的 fallback ROM
        mmu.fallback_rom = mmu.create_enhanced_fallback_rom();
        mmu.rom_state = RomState::Empty;

        mmu
    }

    /// 創建增強的 fallback ROM，包含更多測試指令
    fn create_enhanced_fallback_rom(&self) -> Vec<u8> {
        println!("創建增強版 fallback ROM...");

        let mut fallback = vec![0; 0x8000]; // 32KB 標準大小

        // 重置向量和中斷向量
        fallback[0x0100] = 0x00; // NOP
        fallback[0x0101] = 0xC3; // JP $200
        fallback[0x0102] = 0x00;
        fallback[0x0103] = 0x02;

        // 主程序從 0x200 開始
        let mut pc = 0x200;

        // 初始化PPU
        fallback[pc] = 0x3E;
        pc += 1; // LD A, $91
        fallback[pc] = 0x91;
        pc += 1;
        fallback[pc] = 0xE0;
        pc += 1; // LDH ($40), A (LCDC)
        fallback[pc] = 0x40;
        pc += 1;

        // 清空VRAM的一部分並設置測試圖案
        fallback[pc] = 0x01;
        pc += 1; // LD BC, $8000
        fallback[pc] = 0x00;
        pc += 1;
        fallback[pc] = 0x80;
        pc += 1;

        // 寫入測試圖案到VRAM
        for i in 0..8 {
            fallback[pc] = 0x3E;
            pc += 1; // LD A, pattern
            fallback[pc] = 0xFF - (i * 0x11);
            pc += 1; // 測試圖案
            fallback[pc] = 0x02;
            pc += 1; // LD (BC), A
            fallback[pc] = 0x03;
            pc += 1; // INC BC
        }

        // 設置背景地圖
        fallback[pc] = 0x01;
        pc += 1; // LD BC, $9800
        fallback[pc] = 0x00;
        pc += 1;
        fallback[pc] = 0x98;
        pc += 1;

        for i in 0..32 {
            fallback[pc] = 0x3E;
            pc += 1; // LD A, 0
            fallback[pc] = i % 4;
            pc += 1; // 使用不同的tile
            fallback[pc] = 0x02;
            pc += 1; // LD (BC), A
            fallback[pc] = 0x03;
            pc += 1; // INC BC
        }

        // 無限循環
        fallback[pc] = 0x18;
        pc += 1; // JR $FE (自己跳自己)
        fallback[pc] = 0xFE;
        pc += 1;

        // 設置標準的 Nintendo logo 和校驗和
        // Nintendo logo (簡化版本)
        let nintendo_logo = [
            0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C,
            0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
        ];

        for (i, &byte) in nintendo_logo.iter().enumerate() {
            if 0x104 + i < fallback.len() {
                fallback[0x104 + i] = byte;
            }
        }

        // 計算並設置校驗和
        let mut checksum: u8 = 0;
        for i in 0x134..=0x14C {
            checksum = checksum.wrapping_sub(fallback[i]).wrapping_sub(1);
        }
        fallback[0x14D] = checksum;

        // 設置 cartridge 類型
        fallback[0x147] = 0x00; // ROM ONLY
        fallback[0x148] = 0x00; // 32KB ROM

        println!("增強版 fallback ROM 創建完成 ({} bytes)", fallback.len());
        fallback
    }

    /// 改進的 ROM 載入邏輯
    pub fn load_rom(&mut self, rom_data: Vec<u8>) {
        if self.debug_mode {
            println!(
                "================================================================================"
            );
            println!("開始載入 ROM... (大小: {} bytes)", rom_data.len());
            println!(
                "================================================================================"
            );
        }

        // 重置ROM狀態和信息
        self.rom_state = RomState::Empty;
        self.rom_info = RomInfo::new();
        self.rom_info.size = rom_data.len();

        if rom_data.is_empty() {
            self.handle_empty_rom();
            return;
        }

        // 檢查是否為超小測試ROM（< 16 bytes）
        if rom_data.len() < 16 {
            self.handle_tiny_rom(rom_data);
            return;
        }

        // 檢查是否為測試ROM（16 bytes <= size < 0x150 bytes）
        if rom_data.len() < 0x150 {
            self.handle_test_rom(rom_data);
            return;
        }

        // 標準ROM處理
        self.handle_standard_rom(rom_data);
    }

    fn handle_empty_rom(&mut self) {
        if self.debug_mode {
            println!("⚠️  警告：ROM數據為空，使用增強版 fallback ROM");
        }
        self.rom = self.fallback_rom.clone();
        self.rom_state = RomState::Empty;
        self.rom_info.state = RomState::Empty;
        self.mbc.mbc_type = MBCType::None;
    }

    fn handle_tiny_rom(&mut self, rom_data: Vec<u8>) {
        if self.debug_mode {
            println!("⚠️  警告：ROM太小 (< 16 bytes)，使用增強版 fallback ROM");
            println!("   ROM數據: {:02X?}", rom_data);
        }
        self.rom = self.fallback_rom.clone();
        self.rom_state = RomState::Invalid;
        self.rom_info.state = RomState::Invalid;
        self.mbc.mbc_type = MBCType::None;
    }

    fn handle_test_rom(&mut self, rom_data: Vec<u8>) {
        if self.debug_mode {
            println!("✅ 檢測到測試 ROM (大小: {} bytes)", rom_data.len());
            println!("   跳過標準驗證，直接載入");

            // 顯示ROM的前幾個字節
            print!("   ROM內容: ");
            for (i, &byte) in rom_data.iter().enumerate().take(16) {
                print!("{:02X} ", byte);
                if i == 7 {
                    print!("| ");
                }
            }
            println!();

            // 嘗試解釋指令
            self.analyze_test_rom_instructions(&rom_data);
        }

        self.rom = rom_data;
        self.rom_state = RomState::TestRom;
        self.rom_info.state = RomState::TestRom;
        self.rom_info.is_test_rom = true;
        self.mbc.mbc_type = MBCType::None;

        if self.debug_mode {
            println!("✅ 測試 ROM 載入完成");
        }
    }

    fn handle_standard_rom(&mut self, rom_data: Vec<u8>) {
        if self.debug_mode {
            println!("📦 處理標準 ROM (大小: {} bytes)", rom_data.len());
        }

        self.rom = rom_data;

        // 驗證ROM並設置MBC類型
        if self.validate_and_setup_rom() {
            self.rom_state = RomState::Valid;
            self.rom_info.state = RomState::Valid;
            if self.debug_mode {
                println!("✅ 標準 ROM 載入並驗證成功");
            }
        } else {
            if self.debug_mode {
                println!("⚠️  警告：ROM驗證失敗，使用增強版 fallback ROM");
            }
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Invalid;
            self.rom_info.state = RomState::Invalid;
            self.mbc.mbc_type = MBCType::None;
        }
    }

    /// 分析測試ROM的指令
    fn analyze_test_rom_instructions(&self, rom_data: &[u8]) {
        println!("   📋 指令分析:");

        let mut pc = 0;
        while pc < rom_data.len() {
            if pc >= 16 {
                break;
            } // 只分析前16字節

            let opcode = rom_data[pc];
            let analysis = match opcode {
                0x00 => "NOP - 無操作".to_string(),
                0x01 => {
                    if pc + 2 < rom_data.len() {
                        format!(
                            "LD BC, ${:02X}{:02X} - 載入16位值到BC",
                            rom_data[pc + 2],
                            rom_data[pc + 1]
                        )
                    } else {
                        "LD BC, ???? - 載入16位值到BC (數據不完整)".to_string()
                    }
                }
                0x02 => "LD (BC), A - 將A寫入BC指向的地址".to_string(),
                0x03 => "INC BC - BC遞增".to_string(),
                0x3E => {
                    if pc + 1 < rom_data.len() {
                        format!("LD A, ${:02X} - 載入值到A", rom_data[pc + 1])
                    } else {
                        "LD A, ?? - 載入值到A (數據不完整)".to_string()
                    }
                }
                0xE0 => {
                    if pc + 1 < rom_data.len() {
                        format!(
                            "LDH (${:02X}), A - 將A寫入高位地址FF{:02X}",
                            rom_data[pc + 1],
                            rom_data[pc + 1]
                        )
                    } else {
                        "LDH (??), A - 將A寫入高位地址 (數據不完整)".to_string()
                    }
                }
                0x18 => {
                    if pc + 1 < rom_data.len() {
                        format!("JR ${:02X} - 相對跳轉", rom_data[pc + 1])
                    } else {
                        "JR ?? - 相對跳轉 (數據不完整)".to_string()
                    }
                }
                0xC3 => {
                    if pc + 2 < rom_data.len() {
                        format!(
                            "JP ${:02X}{:02X} - 跳轉到絕對地址",
                            rom_data[pc + 2],
                            rom_data[pc + 1]
                        )
                    } else {
                        "JP ???? - 跳轉到絕對地址 (數據不完整)".to_string()
                    }
                }
                _ => format!("${:02X} - 未知指令或數據", opcode),
            };

            println!("     PC=0x{:04X}: {}", pc, analysis);

            // 移動到下一個指令
            pc += match opcode {
                0x01 | 0xC3 => 3,        // 16位操作數
                0x3E | 0xE0 | 0x18 => 2, // 8位操作數
                _ => 1,                  // 無操作數
            };
        }
    }

    /// 驗證ROM格式並設置MBC控制器（改進版）
    fn validate_and_setup_rom(&mut self) -> bool {
        if self.rom.len() <= 0x147 {
            if self.debug_mode {
                println!("❌ ROM太小，無法讀取cartridge類型");
            }
            return false;
        }

        // 檢查cartridge type
        let cartridge_type = self.rom[0x147];
        self.mbc.mbc_type = match cartridge_type {
            0x00 => MBCType::None,
            0x01..=0x03 => MBCType::MBC1,
            0x05..=0x06 => MBCType::MBC2,
            0x0F..=0x13 => MBCType::MBC3,
            0x19..=0x1E => MBCType::MBC5,
            _ => {
                if self.debug_mode {
                    println!(
                        "⚠️  警告：未知的cartridge類型: 0x{:02X}，使用無MBC模式",
                        cartridge_type
                    );
                }
                MBCType::None
            }
        };

        self.rom_info.mbc_type = self.mbc.mbc_type;

        if self.debug_mode {
            println!(
                "📋 Cartridge類型: 0x{:02X} -> {:?}",
                cartridge_type, self.mbc.mbc_type
            );
        }

        // 驗證Nintendo logo
        if self.rom.len() >= 0x134 {
            self.rom_info.has_nintendo_logo = self.validate_nintendo_logo();
        }

        // 驗證checksum
        if self.rom.len() > 0x14D {
            self.rom_info.checksum_valid = self.validate_checksum();
        }

        // 驗證ROM大小
        if self.rom.len() > 0x148 {
            self.validate_rom_size();
        }

        true
    }

    fn validate_nintendo_logo(&self) -> bool {
        // 簡化的Nintendo logo驗證
        let logo_start = 0x104;
        let logo_end = 0x134;

        if self.rom.len() < logo_end {
            return false;
        }

        // 檢查幾個關鍵字節
        let key_bytes = [(0x104, 0xCE), (0x105, 0xED), (0x106, 0x66), (0x107, 0x66)];

        for (addr, expected) in key_bytes.iter() {
            if self.rom[*addr] != *expected {
                if self.debug_mode {
                    println!("⚠️  Nintendo logo 驗證失敗於地址 0x{:04X}", addr);
                }
                return false;
            }
        }

        if self.debug_mode {
            println!("✅ Nintendo logo 驗證通過");
        }
        true
    }

    fn validate_checksum(&self) -> bool {
        let stored_checksum = self.rom[0x14D];
        let mut calculated: u8 = 0;

        for i in 0x134..=0x14C {
            calculated = calculated.wrapping_sub(self.rom[i]).wrapping_sub(1);
        }

        let valid = stored_checksum == calculated;

        if self.debug_mode {
            if valid {
                println!("✅ Checksum 驗證通過 (0x{:02X})", stored_checksum);
            } else {
                println!(
                    "⚠️  Checksum 驗證失敗。儲存: 0x{:02X}, 計算: 0x{:02X}",
                    stored_checksum, calculated
                );
            }
        }

        valid
    }

    fn validate_rom_size(&self) {
        let rom_size_code = self.rom[0x148];
        let expected_size = match rom_size_code {
            0x00 => 32 * 1024,   // 32KB
            0x01 => 64 * 1024,   // 64KB
            0x02 => 128 * 1024,  // 128KB
            0x03 => 256 * 1024,  // 256KB
            0x04 => 512 * 1024,  // 512KB
            0x05 => 1024 * 1024, // 1MB
            0x06 => 2048 * 1024, // 2MB
            0x07 => 4096 * 1024, // 4MB
            _ => {
                if self.debug_mode {
                    println!("⚠️  未知的ROM大小代碼: 0x{:02X}", rom_size_code);
                }
                self.rom.len()
            }
        };

        if self.debug_mode {
            if self.rom.len() == expected_size {
                println!("✅ ROM大小驗證通過: {} bytes", expected_size);
            } else {
                println!(
                    "⚠️  ROM大小不匹配。預期: {} bytes，實際: {} bytes",
                    expected_size,
                    self.rom.len()
                );
            }
        }
    }

    /// 獲取當前使用的ROM
    fn get_active_rom(&self) -> &Vec<u8> {
        match self.rom_state {
            RomState::Uninitialized | RomState::Empty | RomState::Invalid => &self.fallback_rom,
            RomState::Valid | RomState::TestRom => &self.rom,
        }
    }

    /// 改進的讀取邏輯
    pub fn read_byte(&mut self, addr: u16) -> u8 {
        let result = match addr {
            0x0000..=0x7FFF => self.read_rom_byte(addr),
            0x8000..=0x9FFF => self.read_vram_byte(addr),
            0xFE00..=0xFE9F => self.read_oam_byte(addr),
            0xFF00 => 0xCF, // Joypad
            0xFF04..=0xFF07 => self.timer.read(addr),
            0xFF40..=0xFF4B => self.memory[addr as usize], // PPU registers
            0xFF0F => self.if_reg,
            0xFF10..=0xFF3F => self.apu.borrow().read_reg(addr),
            0xFFFF => self.ie_reg,
            _ => self.memory[addr as usize],
        };

        // ROM讀取計數（調試用）
        if matches!(addr, 0x0000..=0x7FFF) {
            self.rom_read_count += 1;
        }

        result
    }

    fn read_rom_byte(&self, addr: u16) -> u8 {
        let active_rom = self.get_active_rom();

        if active_rom.is_empty() {
            if self.debug_mode {
                println!("🚨 嚴重警告：活動ROM為空! 地址: 0x{:04X}", addr);
            }
            return 0xFF;
        }

        match self.mbc.mbc_type {
            MBCType::None => {
                if (addr as usize) < active_rom.len() {
                    active_rom[addr as usize]
                } else {
                    if self.debug_mode
                        && matches!(self.rom_state, RomState::Valid | RomState::TestRom)
                    {
                        println!(
                            "⚠️  讀取超出ROM範圍! 地址: 0x{:04X}, ROM大小: {}",
                            addr,
                            active_rom.len()
                        );
                    }
                    0xFF
                }
            }
            _ => {
                // MBC處理（與原版相同）
                if (addr as usize) < active_rom.len() {
                    active_rom[addr as usize]
                } else {
                    0xFF
                }
            }
        }
    }

    fn read_vram_byte(&self, addr: u16) -> u8 {
        let vram_addr = (addr - 0x8000) as usize;
        self.vram.borrow()[vram_addr]
    }

    fn read_oam_byte(&self, addr: u16) -> u8 {
        let oam_addr = (addr - 0xFE00) as usize;
        self.oam.borrow()[oam_addr]
    }

    /// 改進的寫入邏輯
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.write_mbc_register(addr, value),
            0x8000..=0x9FFF => self.write_vram_byte(addr, value),
            0xFE00..=0xFE9F => self.write_oam_byte(addr, value),
            0xFF00 => self.joypad.write_joypad_register(value),
            0xFF04..=0xFF07 => self.timer.write(addr, value),
            0xFF40..=0xFF4B => self.write_ppu_register(addr, value),
            0xFF0F => self.if_reg = value,
            0xFF10..=0xFF3F => self.apu.borrow_mut().write_reg(addr, value),
            0xFFFF => self.ie_reg = value,
            _ => self.memory[addr as usize] = value,
        }
    }

    fn write_mbc_register(&mut self, addr: u16, value: u8) {
        // MBC寫入處理（與原版相同）
        match self.mbc.mbc_type {
            MBCType::MBC1 => match addr {
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
            },
            _ => {}
        }
    }

    fn write_vram_byte(&mut self, addr: u16, value: u8) {
        let vram_addr = (addr - 0x8000) as usize;

        // 增強的VRAM寫入調試
        if self.debug_mode && value != 0 {
            println!(
                "📝 VRAM寫入: 地址=0x{:04X} (VRAM+0x{:04X}), 值=0x{:02X}",
                addr, vram_addr, value
            );
            self.vram_write_count += 1;

            // 特別關注tile數據區域的寫入
            if vram_addr < 0x1800 {
                let tile_id = vram_addr / 16;
                let byte_in_tile = vram_addr % 16;
                println!("   → Tile {} 的第 {} 字節", tile_id, byte_in_tile);
            }
        }

        self.vram.borrow_mut()[vram_addr] = value;
    }

    fn write_oam_byte(&mut self, addr: u16, value: u8) {
        let oam_addr = (addr - 0xFE00) as usize;
        self.oam.borrow_mut()[oam_addr] = value;
    }

    fn write_ppu_register(&mut self, addr: u16, value: u8) {
        if addr == 0xFF44 {
            // LY 寄存器是只讀的
            self.memory[addr as usize] = 0;
        } else {
            self.memory[addr as usize] = value;
        }

        if self.debug_mode && addr == 0xFF40 {
            println!("📺 寫入 LCDC: 0x{:02X}", value);
        }
    }

    /// 獲取詳細的ROM信息
    pub fn get_detailed_rom_info(&self) -> String {
        let active_rom = self.get_active_rom();
        format!(
            "================================================================================\n\
             📋 ROM 詳細信息\n\
             ================================================================================\n\
             ROM狀態: {:?}\n\
             ROM大小: {} bytes\n\
             MBC類型: {:?}\n\
             是否為測試ROM: {}\n\
             Nintendo Logo: {}\n\
             Checksum: {}\n\
             使用Fallback: {}\n\
             ROM讀取次數: {}\n\
             VRAM寫入次數: {}\n\
             ================================================================================",
            self.rom_info.state,
            active_rom.len(),
            self.rom_info.mbc_type,
            self.rom_info.is_test_rom,
            if self.rom_info.has_nintendo_logo {
                "✅ 有效"
            } else {
                "❌ 無效"
            },
            if self.rom_info.checksum_valid {
                "✅ 有效"
            } else {
                "❌ 無效"
            },
            matches!(
                self.rom_state,
                RomState::Uninitialized | RomState::Empty | RomState::Invalid
            ),
            self.rom_read_count,
            self.vram_write_count
        )
    }

    /// 設置調試模式
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
        if enabled {
            println!("🔧 調試模式已開啟");
        } else {
            println!("🔧 調試模式已關閉");
        }
    }

    /// 重置統計數據
    pub fn reset_statistics(&mut self) {
        self.rom_read_count = 0;
        self.vram_write_count = 0;
        if self.debug_mode {
            println!("📊 統計數據已重置");
        }
    }

    // 以下是與原版相容的方法...

    pub fn step(&mut self) {
        let timer_interrupt = self.timer.step(4);
        if timer_interrupt {
            self.if_reg |= 0x04;
        }
    }

    pub fn set_joypad(&mut self, value: u8) {
        self.joypad.direction_keys = value & 0x0F;
        self.joypad.action_keys = (value >> 4) & 0x0F;
    }

    pub fn vram(&self) -> Vec<u8> {
        self.vram.borrow().to_vec()
    }

    pub fn oam(&self) -> [u8; 0xA0] {
        *self.oam.borrow()
    }

    pub fn get_apu(&self) -> Rc<RefCell<APU>> {
        self.apu.clone()
    }

    pub fn step_apu(&mut self) {
        self.apu.borrow_mut().step();
    }

    pub fn get_mmu_version(&self) -> &'static str {
        "mmu_improved_v2.0_enhanced"
    }

    /// 全面的系統診斷
    pub fn system_diagnosis(&self) -> String {
        let mut report = String::new();

        report.push_str(
            "================================================================================\n",
        );
        report.push_str("🔍 Game Boy 模擬器系統診斷報告\n");
        report.push_str(
            "================================================================================\n\n",
        );

        // ROM狀態
        report.push_str("📦 ROM 狀態:\n");
        report.push_str(&format!("   狀態: {:?}\n", self.rom_state));
        report.push_str(&format!("   大小: {} bytes\n", self.get_active_rom().len()));
        report.push_str(&format!("   類型: {:?}\n", self.mbc.mbc_type));
        report.push_str(&format!("   測試ROM: {}\n", self.rom_info.is_test_rom));
        report.push_str("\n");

        // 記憶體統計
        report.push_str("💾 記憶體統計:\n");
        let vram_data = self.vram.borrow();
        let non_zero_vram = vram_data.iter().filter(|&&b| b != 0).count();
        report.push_str(&format!(
            "   VRAM 非零字節: {} / {} ({:.1}%)\n",
            non_zero_vram,
            vram_data.len(),
            (non_zero_vram as f64 / vram_data.len() as f64) * 100.0
        ));

        let oam_data = self.oam.borrow();
        let non_zero_oam = oam_data.iter().filter(|&&b| b != 0).count();
        report.push_str(&format!(
            "   OAM 非零字節: {} / {} ({:.1}%)\n",
            non_zero_oam,
            oam_data.len(),
            (non_zero_oam as f64 / oam_data.len() as f64) * 100.0
        ));
        report.push_str("\n");

        // 運行統計
        report.push_str("📊 運行統計:\n");
        report.push_str(&format!("   ROM 讀取次數: {}\n", self.rom_read_count));
        report.push_str(&format!("   VRAM 寫入次數: {}\n", self.vram_write_count));
        report.push_str(&format!(
            "   調試模式: {}\n",
            if self.debug_mode { "開啟" } else { "關閉" }
        ));
        report.push_str("\n");

        // PPU寄存器狀態
        report.push_str("📺 PPU 寄存器:\n");
        report.push_str(&format!(
            "   LCDC (0xFF40): 0x{:02X}\n",
            self.memory[0xFF40]
        ));
        report.push_str(&format!(
            "   STAT (0xFF41): 0x{:02X}\n",
            self.memory[0xFF41]
        ));
        report.push_str(&format!(
            "   SCY  (0xFF42): 0x{:02X}\n",
            self.memory[0xFF42]
        ));
        report.push_str(&format!(
            "   SCX  (0xFF43): 0x{:02X}\n",
            self.memory[0xFF43]
        ));
        report.push_str(&format!(
            "   LY   (0xFF44): 0x{:02X}\n",
            self.memory[0xFF44]
        ));
        report.push_str(&format!(
            "   LYC  (0xFF45): 0x{:02X}\n",
            self.memory[0xFF45]
        ));
        report.push_str("\n");

        // 建議
        report.push_str("💡 建議:\n");
        if matches!(self.rom_state, RomState::Empty | RomState::Invalid) {
            report.push_str("   ⚠️  當前使用 fallback ROM，建議載入有效的測試ROM\n");
        }
        if self.vram_write_count == 0 {
            report.push_str("   ⚠️  VRAM 未收到任何寫入，檢查 CPU 執行狀態\n");
        }
        if non_zero_vram == 0 {
            report.push_str("   ⚠️  VRAM 完全為空，可能需要手動注入測試數據\n");
        }

        report.push_str(
            "\n================================================================================\n",
        );

        report
    }
}
