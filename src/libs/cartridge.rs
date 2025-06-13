#[derive(Debug)]
pub struct CartridgeHeader {
    pub entry_point: [u8; 4],          // 0x100-0x103
    pub nintendo_logo: [u8; 48],       // 0x104-0x133
    pub title: String,                 // 0x134-0x143
    pub cartridge_type: CartridgeType, // 0x147
    pub rom_size: ROMSize,             // 0x148
    pub ram_size: RAMSize,             // 0x149
}

#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,        // 0x00
    MBC1,           // 0x01
    MBC1Ram,        // 0x02
    MBC1RamBattery, // 0x03
    MBC2,           // 0x05
    MBC2Battery,    // 0x06
    Unknown(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum ROMSize {
    KB32,  // 0x00 -  32KB (2 banks)
    KB64,  // 0x01 -  64KB (4 banks)
    KB128, // 0x02 - 128KB (8 banks)
    KB256, // 0x03 - 256KB (16 banks)
    KB512, // 0x04 - 512KB (32 banks)
    MB1,   // 0x05 -   1MB (64 banks)
    MB2,   // 0x06 -   2MB (128 banks)
    Unknown(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum RAMSize {
    None, // 0x00 - No RAM
    KB2,  // 0x01 -  2KB
    KB8,  // 0x02 -  8KB
    KB32, // 0x03 - 32KB (4 banks of 8KB)
    Unknown(u8),
}

impl CartridgeHeader {
    pub fn from_rom(rom: &[u8]) -> Option<Self> {
        if rom.len() < 0x150 {
            return None;
        }

        let mut entry_point = [0u8; 4];
        entry_point.copy_from_slice(&rom[0x100..0x104]);

        let mut nintendo_logo = [0u8; 48];
        nintendo_logo.copy_from_slice(&rom[0x104..0x134]);

        let title = String::from_utf8_lossy(&rom[0x134..0x144])
            .trim_end_matches('\0')
            .to_string();

        let cartridge_type = match rom[0x147] {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::MBC1,
            0x02 => CartridgeType::MBC1Ram,
            0x03 => CartridgeType::MBC1RamBattery,
            0x05 => CartridgeType::MBC2,
            0x06 => CartridgeType::MBC2Battery,
            unknown => CartridgeType::Unknown(unknown),
        };

        let rom_size = match rom[0x148] {
            0x00 => ROMSize::KB32,
            0x01 => ROMSize::KB64,
            0x02 => ROMSize::KB128,
            0x03 => ROMSize::KB256,
            0x04 => ROMSize::KB512,
            0x05 => ROMSize::MB1,
            0x06 => ROMSize::MB2,
            unknown => ROMSize::Unknown(unknown),
        };

        let ram_size = match rom[0x149] {
            0x00 => RAMSize::None,
            0x01 => RAMSize::KB2,
            0x02 => RAMSize::KB8,
            0x03 => RAMSize::KB32,
            unknown => RAMSize::Unknown(unknown),
        };

        Some(CartridgeHeader {
            entry_point,
            nintendo_logo,
            title,
            cartridge_type,
            rom_size,
            ram_size,
        })
    }

    pub fn validate_nintendo_logo(&self) -> bool {
        const NINTENDO_LOGO: [u8; 48] = [
            0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C,
            0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6,
            0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC,
            0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
        ];
        self.nintendo_logo == NINTENDO_LOGO
    }

    pub fn get_rom_size_in_bytes(&self) -> usize {
        match self.rom_size {
            ROMSize::KB32 => 32 * 1024,
            ROMSize::KB64 => 64 * 1024,
            ROMSize::KB128 => 128 * 1024,
            ROMSize::KB256 => 256 * 1024,
            ROMSize::KB512 => 512 * 1024,
            ROMSize::MB1 => 1024 * 1024,
            ROMSize::MB2 => 2 * 1024 * 1024,
            ROMSize::Unknown(_) => 32 * 1024, // 默認最小大小
        }
    }

    pub fn get_ram_size_in_bytes(&self) -> usize {
        match self.ram_size {
            RAMSize::None => 0,
            RAMSize::KB2 => 2 * 1024,
            RAMSize::KB8 => 8 * 1024,
            RAMSize::KB32 => 32 * 1024,
            RAMSize::Unknown(_) => 0,
        }
    }
}
