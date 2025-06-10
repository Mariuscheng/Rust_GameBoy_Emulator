use crate::apu::APU;
use crate::joypad::Joypad;
use crate::timer::Timer;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomState {
    Uninitialized,    // ROM從未載入
    Empty,            // ROM載入但為空
    Invalid,          // ROM載入但格式無效
    Valid,            // ROM載入且有效
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

pub struct MMU {
    memory: [u8; 0x10000],
    pub vram: Rc<RefCell<[u8; 0x2000]>>,
    pub oam: Rc<RefCell<[u8; 0xA0]>>,
    pub rom: Vec<u8>,
    pub rom_state: RomState,
    pub fallback_rom: Vec<u8>,
    pub if_reg: u8,
    pub ie_reg: u8,
    pub joypad: Joypad,
    pub timer: Timer,
    pub apu: Rc<RefCell<APU>>,
    pub mbc: MBCController,
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
        let fallback_rom = Self::create_fallback_rom();
        
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
        };
        
        // 初始化LCD控制暫存器和其他PPU暫存器的預設值
        mmu.memory[0xFF40] = 0x91; // LCDC
        mmu.memory[0xFF41] = 0x85; // STAT
        mmu.memory[0xFF42] = 0x00; // SCY
        mmu.memory[0xFF43] = 0x00; // SCX
        mmu.memory[0xFF44] = 0x00; // LY
        mmu.memory[0xFF45] = 0x00; // LYC
        mmu.memory[0xFF46] = 0x00; // DMA
        mmu.memory[0xFF47] = 0xFC; // BGP
        mmu.memory[0xFF48] = 0xFF; // OBP0
        mmu.memory[0xFF49] = 0xFF; // OBP1
        mmu.memory[0xFF4A] = 0x00; // WY
        mmu.memory[0xFF4B] = 0x00; // WX
        
        mmu
    }
    
    fn create_fallback_rom() -> Vec<u8> {
        let mut fallback = vec![0; 0x8000];
        
        // ROM header區域
        fallback[0x100] = 0x00; // NOP
        fallback[0x101] = 0x18; // JR
        fallback[0x102] = 0xFE; // 無限循環
        
        // 設置基本的ROM標頭
        let title = b"FALLBACK ROM";
        for (i, &byte) in title.iter().enumerate() {
            if i < 16 {
                fallback[0x134 + i] = byte;
            }
        }
        
        fallback[0x147] = 0x00; // Cartridge type
        fallback[0x148] = 0x00; // ROM size
        fallback[0x149] = 0x00; // RAM size
        
        // 簡單的header checksum
        let mut checksum: u8 = 0;
        for i in 0x134..=0x14C {
            checksum = checksum.wrapping_sub(fallback[i]).wrapping_sub(1);
        }
        fallback[0x14D] = checksum;
        
        println!("創建fallback ROM ({} bytes)", fallback.len());
        fallback
    }

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
                    println!("警告：未知的cartridge類型: 0x{:02X}，使用無MBC模式", cartridge_type);
                    MBCType::None
                }
            };
            
            println!("檢測到cartridge類型: 0x{:02X} -> {:?}", cartridge_type, self.mbc.mbc_type);
            return true;
        }
        
        false
    }
    
    fn get_active_rom(&self) -> &Vec<u8> {
        match self.rom_state {
            RomState::Uninitialized | RomState::Empty | RomState::Invalid => &self.fallback_rom,
            RomState::Valid => &self.rom,
        }
    }
    
    pub fn get_rom_info(&self) -> String {
        let active_rom = self.get_active_rom();
        format!(
            "ROM狀態: {:?}\nROM大小: {} bytes\nMBC類型: {:?}\n使用fallback: {}",
            self.rom_state,
            active_rom.len(),
            self.mbc.mbc_type,
            matches!(self.rom_state, RomState::Uninitialized | RomState::Empty | RomState::Invalid)
        )
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                let active_rom = self.get_active_rom();
                
                if active_rom.is_empty() {
                    println!("嚴重警告：活動ROM為空! 地址: 0x{:04X}", addr);
                    return 0xFF;
                }
                
                match self.mbc.mbc_type {
                    MBCType::None => {
                        if (addr as usize) < active_rom.len() {
                            active_rom[addr as usize]
                        } else {
                            if matches!(self.rom_state, RomState::Valid) {
                                println!("警告：讀取超出ROM範圍! 地址: 0x{:04X}, ROM大小: {}", 
                                    addr, active_rom.len());
                            }
                            0xFF
                        }
                    }
                    MBCType::MBC1 => {
                        match addr {
                            0x0000..=0x3FFF => {
                                if (addr as usize) < active_rom.len() {
                                    active_rom[addr as usize]
                                } else {
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
                                    0xFF
                                }
                            }
                            _ => 0xFF,
                        }
                    }
                    _ => {
                        if (addr as usize) < active_rom.len() {
                            active_rom[addr as usize]
                        } else {
                            0xFF
                        }
                    }
                }
            }
            0x8000..=0x9FFF => {
                let vram_addr = (addr - 0x8000) as usize;
                self.vram.borrow()[vram_addr]
            }
            0xFE00..=0xFE9F => {
                let oam_addr = (addr - 0xFE00) as usize;
                self.oam.borrow()[oam_addr]
            }
            0xFF00 => 0xCF, // 暫時返回默認的 joypad 值
            0xFF04..=0xFF07 => self.timer.read(addr),
            0xFF40..=0xFF4B => {
                self.memory[addr as usize]
            },            
            0xFF0F => self.if_reg,
            0xFF10..=0xFF3F => self.apu.borrow().read_reg(addr),
            0xFFFF => self.ie_reg,
            _ => {
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize]
                } else {
                    println!("警告：讀取超出記憶體範圍! 地址: 0x{:04X}", addr);
                    0xFF
                }
            }
        }
    }
    
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => {
                match self.mbc.mbc_type {
                    MBCType::MBC1 => {
                        match addr {
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
                        }
                    }                    
                    _ => {}
                }
            }
            0x8000..=0x9FFF => {
                let vram_addr = (addr - 0x8000) as usize;
                
                #[cfg(debug_assertions)]
                {
                    if value != 0 {
                        println!("VRAM寫入: 地址=0x{:04X} (VRAM+0x{:04X}), 值=0x{:02X}", 
                            addr, vram_addr, value);
                    }
                }
                
                self.vram.borrow_mut()[vram_addr] = value;
            }
            0xFE00..=0xFE9F => {
                let oam_addr = (addr - 0xFE00) as usize;
                self.oam.borrow_mut()[oam_addr] = value;
            }
            0xFF00 => self.joypad.write_joypad_register(value),
            0xFF04..=0xFF07 => self.timer.write(addr, value),
            0xFF40..=0xFF4B => {
                if addr == 0xFF44 {
                    self.memory[addr as usize] = 0;
                } else {
                    self.memory[addr as usize] = value;
                }
                
                #[cfg(debug_assertions)]
                {
                    if addr == 0xFF40 {
                        println!("寫入 LCDC: 0x{:02X}", value);
                    }
                }
            },            
            0xFF0F => self.if_reg = value,
            0xFF10..=0xFF3F => self.apu.borrow_mut().write_reg(addr, value),
            0xFFFF => self.ie_reg = value,
            _ => {
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize] = value;
                } else {
                    println!("警告：寫入超出記憶體範圍! 地址: 0x{:04X}", addr);
                }
            }
        }
    }

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
    
    pub fn read_vram(&self, addr: u16) -> u8 {
        let index = (addr as usize) % 0x2000;
        if index < 0x2000 {
            self.vram.borrow()[index]
        } else {
            println!("警告：VRAM 讀取索引超出範圍! 索引: {}", index);
            0xFF
        }
    }
    
    pub fn write_vram(&mut self, addr: u16, value: u8) {
        let index = (addr as usize) % 0x2000;
        if index < 0x2000 {
            self.vram.borrow_mut()[index] = value;
        } else {
            println!("警告：VRAM 寫入索引超出範圍! 索引: {}", index);
        }
    }

    pub fn vram(&self) -> Vec<u8> {
        let result = self.vram.borrow().to_vec();
        
        #[cfg(debug_assertions)]
        {
            println!("vram() 被調用，返回真實VRAM數據...");
        }
        
        result
    }

    pub fn get_apu(&self) -> Rc<RefCell<APU>> {
        self.apu.clone()
    }

    pub fn debug_fields(&self) {
        println!("MMU debug - checking all fields:");
        println!("- memory array exists");
        println!("- vram: {:?}", self.vram.as_ptr());
        println!("- oam: {:?}", self.oam.as_ptr()); 
        println!("- rom length: {}", self.rom.len());
        println!("- if_reg: 0x{:02X}", self.if_reg);
        println!("- ie_reg: 0x{:02X}", self.ie_reg);
        println!("- joypad exists");
        println!("- timer exists");
        println!("- apu: {:?}", self.apu.as_ptr());
        println!("- mbc exists");
    }
    
    pub fn step_apu(&mut self) {
        self.apu.borrow_mut().step();
    }

    pub fn get_mmu_version(&self) -> &'static str {
        "external_mmu_with_apu_field_v1.0"
    }
    
    pub fn test_method(&self) -> i32 {
        42
    }
    
    pub fn analyze_vram_content(&self) -> String {
        let vram_data = self.vram.borrow();
        let mut analysis = String::new();
        
        analysis.push_str("================================================================================\n");
        analysis.push_str("VRAM 詳細內容分析\n");
        analysis.push_str("================================================================================\n\n");
        
        let mut non_zero_count = 0;
        let mut pattern_diversity = std::collections::HashSet::new();
        
        for &byte in vram_data.iter() {
            if byte != 0 {
                non_zero_count += 1;
            }
            pattern_diversity.insert(byte);
        }
        
        analysis.push_str(&format!("基本統計:\n"));
        analysis.push_str(&format!("  總字節數: {} bytes\n", vram_data.len()));
        analysis.push_str(&format!("  非零字節數: {} ({:.1}%)\n", 
            non_zero_count, 
            (non_zero_count as f64 / vram_data.len() as f64) * 100.0
        ));
        analysis.push_str(&format!("  不同字節值數量: {}\n", pattern_diversity.len()));
        
        analysis
    }

    pub fn save_vram_analysis(&self) {
        let analysis = self.analyze_vram_content();
        let report_path = "vram_analysis_report.txt";
        if let Ok(mut file) = File::create(report_path) {
            let _ = file.write_all(analysis.as_bytes());
            println!("VRAM 分析報告已保存到: {}", report_path);
        } else {
            println!("無法保存 VRAM 分析報告");
        }
    }

    pub fn test_simple_method(&self) -> i32 {
        123
    }
    
    pub fn simple_version(&self) -> &'static str {
        "test_version"
    }
    
    pub fn oam(&self) -> [u8; 0xA0] {
        *self.oam.borrow()
    }
}
