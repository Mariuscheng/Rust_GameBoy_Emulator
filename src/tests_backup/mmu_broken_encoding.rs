use crate::apu::APU;
use crate::joypad::Joypad;
use crate::timer::Timer;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomState {
    Uninitialized,
    Empty,
    Invalid,
    Valid,
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

        println!("🎮 正在創建 Game Boy 測試模式 ROM...");
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

        // 初始化LCD控制暫存器
        mmu.memory[0xFF40] = 0x91; // LCDC
        mmu.memory[0xFF47] = 0xFC; // BGP

        mmu
    }

    fn create_fallback_rom() -> Vec<u8> {
        let mut fallback = vec![0; 0x8000];

        println!("🎮 正在創建 Game Boy 測試模式 ROM...");

        // Entry point
        fallback[0x100] = 0x00; // NOP
        fallback[0x101] = 0x3E; // LD A, 0x91
        fallback[0x102] = 0x91;
        fallback[0x103] = 0xE0; // LDH (0xFF40), A
        fallback[0x104] = 0x40;

        // Set BGP
        fallback[0x105] = 0x3E; // LD A, 0xE4
        fallback[0x106] = 0xE4;
        fallback[0x107] = 0xE0; // LDH (0xFF47), A
        fallback[0x108] = 0x47;

        // Infinite loop
        fallback[0x109] = 0x18; // JR -2
        fallback[0x10A] = 0xFE;

        println!("🎮 測試 ROM 創建完成 ({} bytes)", fallback.len());

        fallback
    }

    pub fn load_rom(&mut self, rom_data: Vec<u8>) {
        if rom_data.is_empty() {
            self.rom = self.fallback_rom.clone();
            self.rom_state = RomState::Empty;
        } else {
            self.rom = rom_data;
            self.rom_state = RomState::Valid;
        }
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                if (addr as usize) < self.rom.len() {
                    self.rom[addr as usize]
                } else {
                    0xFF
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
            0xFF00 => 0xCF,
            0xFF04..=0xFF07 => self.timer.read(addr),
            0xFF40..=0xFF4B => self.memory[addr as usize],
            0xFF0F => self.if_reg,
            0xFF10..=0xFF3F => self.apu.borrow().read_reg(addr),
            0xFFFF => self.ie_reg,
            _ => {
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize]
                } else {
                    0xFF
                }
            }
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => {
                let vram_addr = (addr - 0x8000) as usize;
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
            }
            0xFF0F => self.if_reg = value,
            0xFF10..=0xFF3F => self.apu.borrow_mut().write_reg(addr, value),
            0xFFFF => self.ie_reg = value,
            _ => {
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize] = value;
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

    pub fn vram(&self) -> Vec<u8> {
        self.vram.borrow().to_vec()
    }

    pub fn get_apu(&self) -> Rc<RefCell<APU>> {
        self.apu.clone()
    }

    pub fn oam(&self) -> [u8; 0xA0] {
        *self.oam.borrow()
    }

    /// 手動寫入測試模式到 VRAM（根據 Fix_blank_screen.md 建議）
    pub fn write_test_pattern_to_vram(&mut self) {
        println!("🔧 手動寫入測試模式到 VRAM...");

        let mut vram = self.vram.borrow_mut();

        // First tile: solid black (all 1s)
        for i in 0..16 {
            vram[i] = 0xFF;
        }

        // Second tile: checkerboard
        for i in (16..32).step_by(2) {
            vram[i] = 0xAA;
            vram[i + 1] = 0x55;
        }

        // Third tile: horizontal stripes
        for i in (32..48).step_by(4) {
            vram[i] = 0xFF;
            vram[i + 1] = 0xFF;
            vram[i + 2] = 0x00;
            vram[i + 3] = 0x00;
        }

        // Make first few tiles in BG map point to these test tiles
        for i in 0..10 {
            vram[0x1800 + i] = (i % 3) as u8;
        }

        println!("🔧 測試模式寫入完成:");
        println!("  - Tile 0: 實心黑色");
        println!("  - Tile 1: 棋盤模式");
        println!("  - Tile 2: 水平條紋");
        println!("  - 背景地圖設定為循環使用這些瓦片");
    }
}

    // �t�ΨB�i��k
    pub fn step_apu(&mut self) {
        self.apu.borrow_mut().step();
    }

    pub fn step_timer(&mut self) {
        // Timer step �\��
    }

    pub fn step_joypad(&mut self) {
        // Joypad step �\��
        self.joypad.update();
    }

    pub fn step_serial(&mut self) {
        // Serial communication step �\��
    }

    pub fn step_dma(&mut self) {
        // DMA step �\��
    }

    // ���թM�ոդ�k
    pub fn test_simple_method(&self) -> i32 {
        123
    }

    pub fn simple_version(&self) -> &'static str {
        "clean_version_1.0"
    }

    pub fn get_mmu_version(&self) -> &'static str {
        "clean_mmu_v1.0"
    }

    pub fn test_method(&self) -> i32 {
        42
    }

    pub fn debug_fields(&self) {
        println!("MMU debug - �Ҧ��r�q���`");
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
        format!("VRAM ���R: {} / {} �r�`�D�s", non_zero_count, vram_data.len())
    }

    pub fn save_vram_analysis(&self) {
        println!("VRAM ���R�w�O�s");
    }
}
