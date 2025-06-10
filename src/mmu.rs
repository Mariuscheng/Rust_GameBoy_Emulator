use crate::apu::APU;
use crate::joypad::Joypad;
use crate::timer::Timer;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomState {
    Uninitialized, // ROM從未載入
    Empty,         // ROM載入但為空
    Invalid,       // ROM載入但格式無效
    Valid,         // ROM載入且有效
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

/// ROM 基本信息結構
#[derive(Debug, Clone)]
pub struct RomInfo {
    pub title: String,
    pub rom_type: u8,
    pub rom_size: u8,
    pub ram_size: u8,
    pub is_test_rom: bool,
}

impl RomInfo {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            rom_type: 0,
            rom_size: 0,
            ram_size: 0,
            is_test_rom: false,
        }
    }

    pub fn from_rom(rom_data: &[u8]) -> Self {
        let mut info = Self::new();

        if rom_data.len() >= 0x150 {
            // 提取標題
            let mut title = String::new();
            for i in 0x134..0x144 {
                if i < rom_data.len() {
                    let c = rom_data[i];
                    if c >= 32 && c <= 126 {
                        title.push(c as char);
                    } else {
                        break;
                    }
                }
            }
            info.title = title.trim_end_matches('\0').to_string();

            // 提取ROM類型、大小和RAM大小
            if rom_data.len() > 0x147 {
                info.rom_type = rom_data[0x147];

                if rom_data.len() > 0x148 {
                    info.rom_size = rom_data[0x148];

                    if rom_data.len() > 0x149 {
                        info.ram_size = rom_data[0x149];
                    }
                }
            }
        } else {
            info.is_test_rom = true;
            info.title = "TEST ROM".to_string();
        }

        info
    }
}

pub struct MMU {
    memory: [u8; 0x10000],
    pub vram: Rc<RefCell<[u8; 0x2000]>>,
    pub oam: Rc<RefCell<[u8; 0xA0]>>,
    pub rom: Vec<u8>,
    pub rom_state: RomState,   // 新增ROM狀態追蹤
    pub fallback_rom: Vec<u8>, // 新增fallback ROM
    pub if_reg: u8,
    pub ie_reg: u8,
    pub joypad: Joypad,
    pub timer: Timer,
    pub apu: Rc<RefCell<APU>>,
    pub mbc: MBCController,
    pub rom_info: RomInfo,       // 新增ROM資訊
    pub rom_read_count: usize,   // 計數器
    pub vram_write_count: usize, // 計數器
    pub debug_mode: bool,        // 調試模式
}

impl MMU {
    pub fn new() -> Self {
        let vram = Rc::new(RefCell::new([0; 0x2000]));
        let oam = Rc::new(RefCell::new([0; 0xA0]));
        Self::new_with_vram_oam(vram, oam)
    }

    pub fn new_with_vram_oam(
        vram: Rc<RefCell<[u8; 0x2000]>>,
        oam: Rc<RefCell<[u8; 0xA0]>>,
    ) -> Self {
        let apu = Rc::new(RefCell::new(APU::new()));
        // 創建最小化的fallback ROM
        println!("DEBUG: 開始調用 create_fallback_rom 函數");
        let fallback_rom = Self::create_fallback_rom();
        println!(
            "DEBUG: create_fallback_rom 函數執行完畢，ROM 大小: {}",
            fallback_rom.len()
        );

        let mut mmu = Self {
            memory: [0; 0x10000],
            vram,
            oam,
            rom: Vec::new(),
            rom_state: RomState::Uninitialized,
            fallback_rom,
            if_reg: 0,
            ie_reg: 0,
            joypad: Joypad::new(),
            timer: Timer::new(),
            apu,
            mbc: MBCController::new(MBCType::None),
            rom_info: RomInfo::new(),
            rom_read_count: 0,
            vram_write_count: 0,
            debug_mode: false,
        };

        // 初始化LCD控制暫存器和其他PPU暫存器的預設值
        // 模擬Game Boy啟動後的狀態
        mmu.memory[0xFF40] = 0x91; // LCDC: LCD啟用, BG啟用, BG & Window瓦片數據=$8000-$8FFF, BG瓦片映射=$9800-$9BFF
        mmu.memory[0xFF41] = 0x85; // STAT: LYC=LY中斷啟用, 模式2 OAM中斷啟用
        mmu.memory[0xFF42] = 0x00; // SCY: 滾動Y
        mmu.memory[0xFF43] = 0x00; // SCX: 滾動X
        mmu.memory[0xFF44] = 0x00; // LY: LCD Y坐標
        mmu.memory[0xFF45] = 0x00; // LYC: LY比較
        mmu.memory[0xFF46] = 0x00; // DMA: DMA傳輸
        mmu.memory[0xFF47] = 0xFC; // BGP: BG調色盤
        mmu.memory[0xFF48] = 0xFF; // OBP0: OBJ調色盤0
        mmu.memory[0xFF49] = 0xFF; // OBP1: OBJ調色盤1
        mmu.memory[0xFF4A] = 0x00; // WY: Window Y位置
        mmu.memory[0xFF4B] = 0x00; // WX: Window X位置
                                   // 初始化VRAM為空白，不再注入測試數據
                                   // ROM加載後，CPU執行將會正確寫入VRAM

        mmu
    }

    /// 創建一個功能性的測試 ROM，會寫入 VRAM 數據以驗證顯示
    fn create_fallback_rom() -> Vec<u8> {
        let mut fallback = vec![0; 0x8000];

        println!("🎮 正在創建 Game Boy 測試模式 ROM...");

        // ROM header area (完全按照 Fix_blank_screen.md)
        fallback[0x100] = 0x00; // Entry point: NOP
        fallback[0x101] = 0x3E; // LD A, value        fallback[0x102] = 0x91; // value = 0x91 (LCDC value to enable LCD and BG)

        // Set LCDC register to enable LCD and background
        fallback[0x103] = 0xE0; // LDH (0xFF00+n), A
        fallback[0x104] = 0x40; // n = 0x40 (0xFF40 is LCDC)

        // Set BGP (BG Palette)
        fallback[0x105] = 0x3E; // LD A, value
        fallback[0x106] = 0xE4; // value = 0xE4 (typical GB palette)
        fallback[0x107] = 0xE0; // LDH (0xFF00+n), A
        fallback[0x108] = 0x47; // n = 0x47 (0xFF47 is BGP)

        // Write a simple tile pattern to VRAM
        // First set HL to point to tile data area
        fallback[0x109] = 0x21; // LD HL, nn
        fallback[0x10A] = 0x00; // low byte of 0x8000
        fallback[0x10B] = 0x80; // high byte of 0x8000

        // Write first tile (checkerboard pattern)
        // Tile data takes 16 bytes (2 bytes per row, 8 rows)
        fallback[0x10C] = 0x3E; // LD A, value
        fallback[0x10D] = 0x55; // value = 0x55 (alternating bits)
        fallback[0x10E] = 0x22; // LD (HL+), A
        fallback[0x10F] = 0x3E; // LD A, value
        fallback[0x110] = 0xAA; // value = 0xAA (opposite alternating bits)
        fallback[0x111] = 0x22; // LD (HL+), A

        // Repeat for remaining 7 rows (simplified in this example)
        fallback[0x112] = 0x3E; // LD A, value
        fallback[0x113] = 0xFF; // value = 0xFF (solid row)

        for i in 0..14 {
            fallback[0x114 + i * 2] = 0x22; // LD (HL+), A
        }

        // Write tile ID 1 to background map at position (0,0)
        fallback[0x130] = 0x21; // LD HL, nn
        fallback[0x131] = 0x00; // low byte of 0x9800
        fallback[0x132] = 0x98; // high byte of 0x9800
        fallback[0x133] = 0x3E; // LD A, value
        fallback[0x134] = 0x01; // value = 0x01 (tile ID 1)
        fallback[0x135] = 0x22; // LD (HL+), A

        // Write a few more tiles to make pattern visible
        for i in 0..20 {
            fallback[0x136 + i * 2] = 0x22; // LD (HL+), A
        }

        // Infinite loop
        fallback[0x160] = 0x18; // JR
        fallback[0x161] = 0xFE; // -2 (jump back to self)

        // Standard ROM header data (不覆蓋指令區域)
        let title = b"TEST PATTERN";
        for (i, &byte) in title.iter().enumerate() {
            if i < 12 && (0x170 + i) < 0x180 {
                // 移到安全區域
                fallback[0x170 + i] = byte;
            }
        }

        println!("🎮 測試 ROM 創建完成 ({} bytes)", fallback.len());
        println!("🎮 ROM 將設定 LCDC、BGP 並寫入測試瓦片");

        fallback
    }

    pub fn load_rom(&mut self, rom_data: Vec<u8>) {
        println!("正在載入ROM... (大小: {} bytes)", rom_data.len());
        println!("DEBUG: 進入 load_rom 函數");

        // 重置ROM狀態
        self.rom_state = RomState::Empty;

        if rom_data.is_empty() {
            println!("警告：ROM數據為空，將使用fallback ROM");
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Empty;
            self.mbc.mbc_type = MBCType::None;
            self.rom_info = RomInfo::new();
            self.rom_info.is_test_rom = true;
            return;
        }

        println!("DEBUG: ROM數據不為空，檢查大小限制");
        // 檢查ROM最小大小
        // 對於測試 ROM，如果太小則使用功能性 fallback ROM
        if rom_data.len() < 20 {
            println!("警告：ROM太小 (< 20 bytes)，將使用功能性測試 ROM");
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Invalid;
            self.mbc.mbc_type = MBCType::None;
            self.rom_info = RomInfo::new();
            self.rom_info.is_test_rom = true;
            return;
        }

        println!("DEBUG: ROM大小 >= 16 bytes，檢查是否為測試ROM");

        // 如果是小型測試 ROM（< 0x150 bytes），直接使用，跳過標準驗證
        if rom_data.len() < 0x150 {
            println!(
                "檢測到測試 ROM (大小: {} bytes)，跳過標準驗證",
                rom_data.len()
            );
            println!("DEBUG: 設置測試ROM為主ROM");
            self.rom = rom_data;
            self.rom_state = RomState::Valid;
            self.mbc.mbc_type = MBCType::None;
            self.rom_info = RomInfo::new();
            self.rom_info.is_test_rom = true;
            println!("DEBUG: 測試ROM載入完成，狀態: {:?}", self.rom_state);
            return;
        }

        println!("DEBUG: ROM大小 >= 0x150，進入標準驗證流程");

        self.rom = rom_data.clone();
        self.rom_info = RomInfo::from_rom(&rom_data);

        // 驗證ROM並設置MBC類型
        if self.validate_and_setup_rom() {
            self.rom_state = RomState::Valid;
            println!("ROM載入成功，狀態: {:?}", self.rom_state);
        } else {
            println!("警告：ROM驗證失敗，將使用fallback ROM");
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Invalid;
            self.mbc.mbc_type = MBCType::None;
            self.rom_info.is_test_rom = true;
        }
    }

    /// 驗證ROM格式並設置MBC控制器
    fn validate_and_setup_rom(&mut self) -> bool {
        // 檢查cartridge type
        if self.rom.len() > 0x147 {
            let cartridge_type = self.rom[0x147];
            self.mbc.mbc_type = match cartridge_type {
                0x00 => MBCType::None,
                0x01..=0x03 => MBCType::MBC1,
                0x05..=0x06 => MBCType::MBC2,
                0x0F..=0x13 => MBCType::MBC3,
                0x19..=0x1E => MBCType::MBC5,
                _ => {
                    println!(
                        "警告：未知的cartridge類型: 0x{:02X}，使用無MBC模式",
                        cartridge_type
                    );
                    MBCType::None
                }
            };

            println!(
                "檢測到cartridge類型: 0x{:02X} -> {:?}",
                cartridge_type, self.mbc.mbc_type
            );
            // 驗證ROM大小
            if self.rom.len() > 0x148 {
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
                        println!("警告：未知的ROM大小代碼: 0x{:02X}", rom_size_code);
                        self.rom.len()
                    }
                };

                if self.rom.len() != expected_size {
                    println!(
                        "警告：ROM大小不匹配。預期: {} bytes，實際: {} bytes",
                        expected_size,
                        self.rom.len()
                    );
                }
            }

            return true;
        }

        false
    }

    /// 獲取當前使用的ROM（可能是原始ROM或fallback ROM）
    fn get_active_rom(&self) -> &Vec<u8> {
        match self.rom_state {
            RomState::Uninitialized | RomState::Empty | RomState::Invalid => &self.fallback_rom,
            RomState::Valid => &self.rom,
        }
    }

    /// 提供ROM狀態資訊
    pub fn get_rom_info(&self) -> String {
        let active_rom = self.get_active_rom();
        format!(
            "ROM狀態: {:?}\n\
             ROM大小: {} bytes\n\
             MBC類型: {:?}\n\
             使用fallback: {}",
            self.rom_state,
            active_rom.len(),
            self.mbc.mbc_type,
            matches!(
                self.rom_state,
                RomState::Uninitialized | RomState::Empty | RomState::Invalid
            )
        )
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        self.rom_read_count += 1;

        match addr {
            0x0000..=0x7FFF => {
                let active_rom = self.get_active_rom();

                // 如果ROM未初始化，使用fallback ROM
                if active_rom.is_empty() {
                    if self.debug_mode {
                        println!("嚴重警告：活動ROM為空! 地址: 0x{:04X}", addr);
                    }
                    return 0xFF;
                }

                match self.mbc.mbc_type {
                    MBCType::None => {
                        if (addr as usize) < active_rom.len() {
                            active_rom[addr as usize]
                        } else {
                            if self.debug_mode && matches!(self.rom_state, RomState::Valid) {
                                println!(
                                    "警告：讀取超出ROM範圍! 地址: 0x{:04X}, ROM大小: {}",
                                    addr,
                                    active_rom.len()
                                );
                            }
                            0xFF
                        }
                    }
                    MBCType::MBC1 => match addr {
                        0x0000..=0x3FFF => {
                            if (addr as usize) < active_rom.len() {
                                active_rom[addr as usize]
                            } else {
                                if self.debug_mode && matches!(self.rom_state, RomState::Valid) {
                                    println!("警告：MBC1模式下讀取超出ROM範圍! 地址: 0x{:04X}, ROM大小: {}", 
                                            addr, active_rom.len());
                                }
                                0xFF
                            }
                        }
                        0x4000..=0x7FFF => {
                            let bank = self.mbc.rom_bank as usize;
                            let base_addr = bank * 0x4000;
                            let offset = addr as usize - 0x4000;
                            let real_addr = base_addr + offset;

                            if real_addr < active_rom.len() {
                                active_rom[real_addr]
                            } else {
                                if self.debug_mode && matches!(self.rom_state, RomState::Valid) {
                                    println!(
                                        "警告：MBC1模式下ROM bank超出範圍! Bank: {}, 地址: 0x{:04X}, ROM大小: {}",
                                        bank, addr, active_rom.len()
                                    );
                                }
                                0xFF
                            }
                        }
                        _ => unreachable!(),
                    },
                    // 簡化其他MBC類型的處理（這裡僅保留通用模式）
                    _ => {
                        if (addr as usize) < active_rom.len() {
                            active_rom[addr as usize]
                        } else {
                            if self.debug_mode && matches!(self.rom_state, RomState::Valid) {
                                println!(
                                    "警告：未完全支持的MBC模式讀取ROM! 地址: 0x{:04X}, ROM大小: {}",
                                    addr,
                                    active_rom.len()
                                );
                            }
                            0xFF
                        }
                    }
                }
            }
            0x8000..=0x9FFF => {
                let vram = self.vram.borrow();
                vram[(addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                // External RAM
                match self.mbc.mbc_type {
                    MBCType::MBC1 => {
                        if !self.mbc.ram_enabled {
                            return 0xFF;
                        }
                        // 簡單返回0以保持功能性
                        0
                    }
                    // 對於其他類型，簡單返回0xFF
                    _ => 0xFF,
                }
            }
            0xC000..=0xFDFF => {
                // Internal RAM + Echo
                let addr = if addr >= 0xE000 {
                    // Echo of internal RAM
                    addr - 0x2000
                } else {
                    addr
                };
                self.memory[addr as usize]
            }
            0xFE00..=0xFE9F => {
                // OAM
                let oam = self.oam.borrow();
                oam[(addr - 0xFE00) as usize]
            }
            0xFF00 => {
                // Joypad register
                let mut value = 0xCF; // 高4位總是1
                if self.joypad.select_direction {
                    value &= self.joypad.direction_keys;
                }
                if self.joypad.select_action {
                    value &= self.joypad.action_keys;
                }
                value
            }
            0xFF01..=0xFF0E => self.memory[addr as usize],
            0xFF0F => self.if_reg, // Interrupt flag
            0xFF10..=0xFF3F => {
                // Audio registers
                if addr >= 0xFF10 && addr <= 0xFF3F {
                    self.apu.borrow().read_reg(addr)
                } else {
                    self.memory[addr as usize]
                }
            }
            0xFF40..=0xFF7F => {
                // IO Registers
                self.memory[addr as usize]
            }
            0xFF80..=0xFFFE => {
                // High RAM
                self.memory[addr as usize]
            }
            0xFFFF => self.ie_reg, // Interrupt enable
            _ => {
                // Unmapped memory region
                0xFF
            }
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => match self.mbc.mbc_type {
                MBCType::None => {
                    // 不可寫入ROM區域，忽略
                }
                MBCType::MBC1 => match addr {
                    // RAM啟用/禁用 (0x0000-0x1FFF)
                    0x0000..=0x1FFF => {
                        self.mbc.ram_enabled = (value & 0x0F) == 0x0A;
                    }
                    // ROM Bank選擇 (0x2000-0x3FFF)
                    0x2000..=0x3FFF => {
                        let mut bank = value & 0x1F;
                        if bank == 0 {
                            bank = 1;
                        }
                        self.mbc.rom_bank = (self.mbc.rom_bank & 0x60) | bank;
                    }
                    // RAM Bank選擇 (0x4000-0x5FFF)
                    0x4000..=0x5FFF => {
                        if self.mbc.mbc1_mode == 0 {
                            self.mbc.rom_bank = (self.mbc.rom_bank & 0x1F) | ((value & 0x03) << 5);
                        } else {
                            self.mbc.ram_bank = value & 0x03;
                        }
                    }
                    // ROM/RAM Mode選擇 (0x6000-0x7FFF)
                    0x6000..=0x7FFF => {
                        self.mbc.mbc1_mode = value & 0x01;
                    }
                    _ => {}
                },
                // 對於其他MBC類型，這裡略過完整實現
                _ => {}
            },
            0x8000..=0x9FFF => {
                // VRAM
                let mut vram = self.vram.borrow_mut();
                vram[(addr - 0x8000) as usize] = value;
                self.vram_write_count += 1;
            }
            0xA000..=0xBFFF => {
                // External RAM (currently not implemented)
            }
            0xC000..=0xFDFF => {
                // Internal RAM + Echo
                let addr = if addr >= 0xE000 {
                    // Echo of internal RAM
                    addr - 0x2000
                } else {
                    addr
                };
                self.memory[addr as usize] = value;
            }
            0xFE00..=0xFE9F => {
                // OAM
                let mut oam = self.oam.borrow_mut();
                oam[(addr - 0xFE00) as usize] = value;
            }
            0xFF00 => {
                // Joypad register
                self.joypad.select_direction = (value & 0x10) == 0;
                self.joypad.select_action = (value & 0x20) == 0;
            }
            0xFF01..=0xFF0E => {
                self.memory[addr as usize] = value;
            }
            0xFF0F => {
                // Interrupt Flag
                self.if_reg = value;
            }
            0xFF10..=0xFF3F => {
                // Audio registers
                if addr >= 0xFF10 && addr <= 0xFF3F {
                    self.apu.borrow_mut().write_reg(addr, value);
                }
                self.memory[addr as usize] = value;
            }
            0xFF40..=0xFF7F => {
                // IO Registers
                self.memory[addr as usize] = value;
            }
            0xFF80..=0xFFFE => {
                // High RAM
                self.memory[addr as usize] = value;
            }
            0xFFFF => {
                // Interrupt Enable
                self.ie_reg = value;
            }
            _ => {
                // Unmapped memory region - ignore writes
            }
        }
    }

    pub fn get_if(&self) -> u8 {
        self.if_reg
    }

    pub fn set_if(&mut self, value: u8) {
        self.if_reg = value;
    }

    pub fn get_ie(&self) -> u8 {
        self.ie_reg
    }

    pub fn set_ie(&mut self, value: u8) {
        self.ie_reg = value;
    }

    pub fn reset_interrupts(&mut self, flags: u8) {
        self.if_reg &= !flags;
    }
    pub fn read_word(&mut self, addr: u16) -> u16 {
        let low = self.read_byte(addr) as u16;
        let high = self.read_byte(addr + 1) as u16;
        (high << 8) | low
    }

    pub fn write_word(&mut self, addr: u16, value: u16) {
        self.write_byte(addr, (value & 0xFF) as u8);
        self.write_byte(addr + 1, (value >> 8) as u8);
    }

    pub fn update_joypad(&mut self, button_id: u8, pressed: bool) {
        // Convert boolean pressed to appropriate u8 values for direction_keys and action_keys
        let (direction_keys, action_keys) = match pressed {
            // When pressed, use current values with button_id bit cleared (0 = pressed)
            true => {
                let direction = if button_id <= 3 {
                    self.joypad.direction_keys & !(1 << button_id)
                } else {
                    self.joypad.direction_keys
                };
                let action = if button_id >= 4 {
                    self.joypad.action_keys & !(1 << (button_id - 4))
                } else {
                    self.joypad.action_keys
                };
                (direction, action)
            }
            // When released, use current values with button_id bit set (1 = released)
            false => {
                let direction = if button_id <= 3 {
                    self.joypad.direction_keys | (1 << button_id)
                } else {
                    self.joypad.direction_keys
                };
                let action = if button_id >= 4 {
                    self.joypad.action_keys | (1 << (button_id - 4))
                } else {
                    self.joypad.action_keys
                };
                (direction, action)
            }
        };

        self.joypad.update_button(direction_keys, action_keys);
    }

    pub fn get_joypad_state(&self) -> (u8, u8) {
        (self.joypad.direction_keys, self.joypad.action_keys)
    }

    pub fn set_joypad_state(&mut self, directions: u8, actions: u8) {
        self.joypad.direction_keys = directions;
        self.joypad.action_keys = actions;
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

    pub fn read_vram(&self, addr: u16) -> u8 {
        let index = (addr as usize) % 0x2000;
        self.vram.borrow()[index]
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        let index = (addr as usize) % 0x2000;
        self.vram.borrow_mut()[index] = value;
    }

    pub fn analyze_vram_content(&self) -> String {
        let vram_data = self.vram.borrow();
        let non_zero_count = vram_data.iter().filter(|&&b| b != 0).count();
        format!(
            "VRAM 分析: {} / {} 字節非零",
            non_zero_count,
            vram_data.len()
        )
    }

    // 系統步進方法
    pub fn step_apu(&mut self) {
        self.apu.borrow_mut().step();
    }

    pub fn step_timer(&mut self) {
        // Timer step 功能 - 呼叫內部 timer 的 step
        // 注意：這裡只是記錄呼叫，實際 timer 在 main loop 中處理
    }

    pub fn step_joypad(&mut self) {
        // Joypad step 功能
        self.joypad.update();
    }

    pub fn step_serial(&mut self) {
        // Serial communication step 功能
        // 實作串列通信的步進邏輯
    }

    pub fn step_dma(&mut self) {
        // DMA step 功能
        // 實作 DMA 傳輸的步進邏輯
    }

    pub fn step(&mut self) {
        // Combined step function for all components
        self.step_timer();
        self.step_apu();
        self.step_joypad();
        self.step_serial();
        self.step_dma();
    }

    // 測試和調試方法
    pub fn test_simple_method(&self) -> i32 {
        123
    }

    pub fn get_mmu_version(&self) -> &'static str {
        "clean_mmu_v2.0"
    }
    /// 手動寫入測試模式到 VRAM（根據 Fix_blank_screen.md 建議）
    pub fn write_test_pattern_to_vram(&mut self) {
        println!("🔧 手動寫入測試模式到 VRAM (正確 write_byte 方式)...");

        // 使用 write_byte 方法而不是直接訪問 vram 陣列

        // First tile: solid black (all 1s)
        for i in 0..16 {
            self.write_byte(0x8000 + i as u16, 0xFF);
        }

        // Second tile: checkerboard
        for i in (16..32).step_by(2) {
            self.write_byte(0x8000 + i as u16, 0xAA);
            self.write_byte(0x8000 + (i + 1) as u16, 0x55);
        }

        // Third tile: horizontal stripes
        for i in (32..48).step_by(4) {
            self.write_byte(0x8000 + i as u16, 0xFF);
            self.write_byte(0x8000 + (i + 1) as u16, 0xFF);
            self.write_byte(0x8000 + (i + 2) as u16, 0x00);
            self.write_byte(0x8000 + (i + 3) as u16, 0x00);
        }

        // Make first few tiles in BG map point to these test tiles
        for i in 0..10 {
            self.write_byte(0x9800 + i as u16, (i % 3) as u8); // 使用前3個測試瓦片
        }

        println!("🔧 測試模式寫入完成:");
        println!("  - Tile 0: 實心黑色");
        println!("  - Tile 1: 棋盤模式");
        println!("  - Tile 2: 水平條紋");
        println!("  - 背景地圖設定為循環使用這些瓦片");
    }
}
