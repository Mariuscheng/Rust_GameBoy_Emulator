use crate::apu::APU;
use crate::joypad::Joypad;
use crate::timer::Timer;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

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
        let mut mmu = Self {
            memory: [0; 0x10000],
            vram,
            oam,
            rom: Vec::new(),
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

        // 添加測試圖形數據到 VRAM
        let test_tile_data = [
            0xFF, 0x00, 0x7E, 0x00, 0x3C, 0x00, 0x18, 0x00, 0x18, 0x00, 0x3C, 0x00, 0x7E, 0x00,
            0xFF, 0x00,
        ];

        {
            let mut vram_borrow = mmu.vram.borrow_mut();
            for (i, &byte) in test_tile_data.iter().enumerate() {
                vram_borrow[i] = byte;
            }

            for i in 0x1800..0x1820 {
                vram_borrow[i] = 0;
            }
        }

        mmu
    }

    pub fn load_rom(&mut self, rom_data: Vec<u8>) {
        self.rom = rom_data;

        if self.rom.len() > 0x147 {
            let cartridge_type = self.rom[0x147];
            self.mbc.mbc_type = match cartridge_type {
                0x00 => MBCType::None,
                0x01..=0x03 => MBCType::MBC1,
                0x05..=0x06 => MBCType::MBC2,
                0x0F..=0x13 => MBCType::MBC3,
                0x19..=0x1E => MBCType::MBC5,
                _ => MBCType::None,
            };
            println!(
                "檢測到卡帶類型: 0x{:02X} -> {:?}",
                cartridge_type, self.mbc.mbc_type
            );
        }
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                if self.rom.is_empty() {
                    return 0xFF;
                }

                match self.mbc.mbc_type {
                    MBCType::None => {
                        if (addr as usize) < self.rom.len() {
                            self.rom[addr as usize]
                        } else {
                            0xFF
                        }
                    }
                    MBCType::MBC1 => match addr {
                        0x0000..=0x3FFF => {
                            if (addr as usize) < self.rom.len() {
                                self.rom[addr as usize]
                            } else {
                                0xFF
                            }
                        }
                        0x4000..=0x7FFF => {
                            let bank = self.mbc.rom_bank as usize;
                            let real_addr =
                                ((bank * 0x4000) + (addr as usize - 0x4000)) % self.rom.len();
                            self.rom[real_addr]
                        }
                        _ => 0xFF,
                    },
                    _ => {
                        if (addr as usize) < self.rom.len() {
                            self.rom[addr as usize]
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
            0xFF00 => self.joypad.read_joypad_register(0x00),
            0xFF04..=0xFF07 => self.timer.read(addr),
            0xFF40..=0xFF4B => self.memory[addr as usize],
            0xFF0F => self.if_reg,
            0xFF10..=0xFF3F => self.apu.borrow().read_reg(addr),
            0xFFFF => self.ie_reg,
            _ => self.memory[addr as usize],
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => match self.mbc.mbc_type {
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
            },
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
            _ => self.memory[addr as usize] = value,
        }
    }

    pub fn step(&mut self) {
        let timer_interrupt = self.timer.step(4);
        if timer_interrupt {
            self.if_reg |= 0x04;
        }
    }

    pub fn set_joypad(&mut self, value: u8) {
        // 根據 value 設置方向鍵和動作鍵狀態
        self.joypad.direction_keys = value & 0x0F;
        self.joypad.action_keys = (value >> 4) & 0x0F;
    }

    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram.borrow()[(addr as usize) % 0x2000]
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        let index = (addr as usize) % 0x2000;
        self.vram.borrow_mut()[index] = value;
    }

    pub fn vram(&self) -> Vec<u8> {
        println!("vram() 被調用，正在注入測試數據...");
        self.ensure_test_data();
        let result = self.vram.borrow().to_vec();

        println!("正在進行 VRAM 內容分析...");
        let analysis = self.analyze_vram_content();
        println!("{}", analysis);

        self.save_vram_analysis();

        println!("VRAM 前16字節: ");
        for i in 0..16 {
            print!("{:02X} ", result[i]);
        }
        println!();
        result
    }

    pub fn ensure_test_data(&self) {
        println!("正在注入測試數據到 VRAM...");
        let test_tile_data = [
            0xFF, 0x00, 0x7E, 0x00, 0x3C, 0x00, 0x18, 0x00, 0x18, 0x00, 0x3C, 0x00, 0x7E, 0x00,
            0xFF, 0x00,
        ];

        let mut vram_borrow = self.vram.borrow_mut();
        for (i, &byte) in test_tile_data.iter().enumerate() {
            vram_borrow[i] = byte;
        }

        for i in 0x1800..0x1820 {
            vram_borrow[i] = 0;
        }
        println!(
            "測試數據注入完成。前4字節: {:02X} {:02X} {:02X} {:02X}",
            vram_borrow[0], vram_borrow[1], vram_borrow[2], vram_borrow[3]
        );
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

        analysis.push_str(
            "================================================================================\n",
        );
        analysis.push_str("VRAM 詳細內容分析\n");
        analysis.push_str(
            "================================================================================\n\n",
        );

        // 基本統計
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
        analysis.push_str(&format!(
            "  非零字節數: {} ({:.1}%)\n",
            non_zero_count,
            (non_zero_count as f64 / vram_data.len() as f64) * 100.0
        ));
        analysis.push_str(&format!("  不同字節值數量: {}\n", pattern_diversity.len()));
        analysis.push_str("\n");

        // Tile 數據分析
        analysis.push_str("Tile 數據區域分析 (0x8000-0x97FF):\n");
        let mut active_tiles = 0;
        let mut tile_patterns = Vec::new();

        for tile_id in 0..384 {
            let start_addr = tile_id * 16;
            let end_addr = start_addr + 16;

            if end_addr <= vram_data.len() {
                let tile_data = &vram_data[start_addr..end_addr];
                let has_data = tile_data.iter().any(|&b| b != 0);

                if has_data {
                    active_tiles += 1;
                    if tile_patterns.len() < 5 {
                        tile_patterns.push((tile_id, tile_data));
                    }
                }
            }
        }

        analysis.push_str(&format!("  活躍 tiles 數量: {} / 384\n", active_tiles));

        if !tile_patterns.is_empty() {
            analysis.push_str("  前幾個活躍 tiles 的圖案:\n");
            for (tile_id, tile_data) in tile_patterns {
                analysis.push_str(&format!("    Tile {}: ", tile_id));
                for &byte in tile_data.iter().take(8) {
                    analysis.push_str(&format!("{:02X} ", byte));
                }
                analysis.push_str("...\n");

                analysis.push_str("      圖案預覽:\n");
                for row in 0..8 {
                    analysis.push_str("        ");
                    let low_byte = tile_data[row * 2];
                    let high_byte = tile_data[row * 2 + 1];

                    for bit in (0..8).rev() {
                        let low_bit = (low_byte >> bit) & 1;
                        let high_bit = (high_byte >> bit) & 1;
                        let pixel = (high_bit << 1) | low_bit;

                        let char = match pixel {
                            0 => "⬜",
                            1 => "🔲",
                            2 => "⬛",
                            3 => "⚫",
                            _ => "?",
                        };
                        analysis.push_str(char);
                    }
                    analysis.push_str("\n");
                }
            }
        }
        analysis.push_str("\n");

        analysis.push_str(
            "================================================================================\n",
        );
        analysis
    }

    pub fn save_vram_analysis(&self) {
        let analysis = self.analyze_vram_content();
        let report_path = "c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\vram_analysis_report.txt";
        if let Ok(mut file) = File::create(report_path) {
            let _ = file.write_all(analysis.as_bytes());
            println!("VRAM 分析報告已保存到: {}", report_path);
        } else {
            println!("無法保存 VRAM 分析報告");
        }
    }

    // 測試方法
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
