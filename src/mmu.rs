pub struct MMU {
    memory: [u8; 0x10000], // 64KB
    rom_bank: u8,          // 目前選擇的 ROM bank
    ram_bank: u8,
    mbc1_mode: u8, // 0: ROM banking, 1: RAM banking
    pub vram: [u8; 0x2000],
    pub rom: Vec<u8>,
    pub if_reg: u8, // 0xFF0F
    pub ie_reg: u8, // 0xFFFF
    pub joypad: u8,
    pub bgp: u8,
}

impl MMU {
    pub fn new() -> Self {
        let mut memory = [0u8; 0x10000];
        memory[0xFF47] = 0xFC; // BGP 預設 Game Boy 開機值
        Self {
            memory,
            rom: Vec::new(),
            rom_bank: 1,
            ram_bank: 0,
            mbc1_mode: 0,
            vram: [0; 0x2000],
            if_reg: 0,
            ie_reg: 0,
            joypad: 0xFF,
            bgp: 0xFC,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.rom = rom.to_vec();
        let len = rom.len().min(0x4000);
        // 只把 bank 0 複製到 0x0000~0x3FFF
        self.memory[0..len].copy_from_slice(&rom[0..len]);
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let bank = if self.mbc1_mode == 0 {
                    self.rom_bank & 0x7F
                } else {
                    (self.rom_bank & 0x1F) | ((self.ram_bank & 0x03) << 5)
                };
                let offset = (bank as usize) * 0x4000 + (addr as usize - 0x4000);
                self.rom.get(offset).copied().unwrap_or(0xFF)
            }
            0xFF0F => self.if_reg,   // IF register
            0xFFFF => self.ie_reg,   // IE register
            _ => self.memory[addr as usize],
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        match addr {
            0x2000..=0x3FFF => {
                // 設定 ROM bank 低 5 位
                let mut bank = (self.rom_bank & 0x60) | (value & 0x1F);
                if bank & 0x1F == 0 { bank |= 1; }
                self.rom_bank = bank;
            }
            0x4000..=0x5FFF => {
                // 設定 ROM bank 高 2 位或 RAM bank
                if self.mbc1_mode == 0 {
                    // ROM banking mode
                    self.rom_bank = (self.rom_bank & 0x1F) | ((value & 0x03) << 5);
                } else {
                    // RAM banking mode
                    self.ram_bank = value & 0x03;
                }
            }
            0x6000..=0x7FFF => {
                // Mode select
                self.mbc1_mode = value & 0x01;
            }
            0x8000..=0x9FFF => {
                self.memory[addr as usize] = value;
                self.vram[addr as usize - 0x8000] = value;
                // println!("VRAM write: addr={:04X} value={:02X}", addr, value);
            }
            0xFF0F => {
                self.if_reg = value;
            }
            0xFFFF => {
                self.ie_reg = value;
            }
            0xFF47 => {
                self.memory[addr as usize] = value;
                // 假設你有 let ppu = &mut self.ppu;
                // ppu.bgp = value;
            }
            0xFF48 => {
                self.memory[addr as usize] = value;
                // ppu.obp0 = value;
            }
            0xFF49 => {
                self.memory[addr as usize] = value;
                // ppu.obp1 = value;
            }
            0xFF00..=0xFF7F | 0xFF80..=0xFFFE => {
                self.memory[addr as usize] = value;
            }
            _ => {
                self.memory[addr as usize] = value;
            }
        }


    }

    pub fn set_joypad(&mut self, value: u8) {
        self.joypad = value;
    }

    #[allow(dead_code)]
    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr as usize) % 0x2000]
    }

    #[allow(dead_code)]
    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr as usize) % 0x2000] = value;
    }

    #[allow(dead_code)]
    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    #[allow(dead_code)]
    pub fn vram(&self) -> &[u8] {
        &self.vram
    }

    #[allow(dead_code)]
    pub fn bg_map(&self) -> &[u8] {
        &self.memory[0x9800..0x9C00]
    }

    #[allow(dead_code)]
    pub fn oam(&self) -> &[u8] {
        &self.memory[0xFE00..0xFEA0]
    }

    #[allow(dead_code)]
    pub fn get_if(&self) -> u8 {
        self.if_reg
    }
    #[allow(dead_code)]
    pub fn get_ie(&self) -> u8 {
        self.ie_reg
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    #[allow(dead_code)]
    pub fn write_reg(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }

    pub fn get_oam(&self) -> &[u8] {
        &self.memory[0xFE00..0xFEA0]
    }

    // 在每次 CPU step 時呼叫
    #[allow(dead_code)]
    pub fn step(&mut self) {
        // 簡單模擬 DIV 增加
        self.memory[0xFF04] = self.memory[0xFF04].wrapping_add(1);
    }
}

