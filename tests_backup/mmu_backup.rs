pub struct MMU {
    memory: [u8; 0x10000], // 64KB
    rom: Vec<u8>,          // 儲存整個 ROM
    rom_bank: u8,          // 目前選擇的 ROM bank
    ram_bank: u8,
    mbc1_mode: u8, // 0: ROM banking, 1: RAM banking
    pub vram: [u8; 0x2000],
}

impl MMU {
    pub fn new() -> Self {
        Self {
            memory: [0; 0x10000],
            rom: Vec::new(),
            rom_bank: 1,
            ram_bank: 0,
            mbc1_mode: 0,
            vram: [0; 0x2000],
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
            _ => self.memory[addr as usize],
        }
    }

    #[allow(dead_code)]
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
            }
            0xFF00..=0xFF7F | 0xFF80..=0xFFFF => {
                self.memory[addr as usize] = value;
            }
            _ => {
                self.memory[addr as usize] = value;
            }
        }
    }

    #[allow(dead_code)]
    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr as usize) % 0x2000]
    }

    #[allow(dead_code)]
    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr as usize) % 0x2000] = value;
    }

    pub fn memory(&self) -> &[u8] {
        &self.memory
    }
    pub fn vram(&self) -> &[u8] {
        &self.memory[0x8000..0xA000]
    }
    pub fn bg_map(&self) -> &[u8] {
        &self.memory[0x9800..0x9C00]
    }
    pub fn oam(&self) -> &[u8] {
        &self.memory[0xFE00..0xFEA0]
    }
}