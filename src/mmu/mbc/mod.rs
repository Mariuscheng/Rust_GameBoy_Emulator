mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;
mod types;

use mbc1::MBC1;
use mbc2::MBC2;
use mbc3::MBC3;
use mbc5::MBC5;
pub use types::*;

/// MBC 控制器特徵
pub trait MBCController {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

/// 創建合適的 MBC 控制器
pub fn create_mbc_controller(rom_data: Vec<u8>) -> Box<dyn MBCController> {
    if rom_data.len() < 0x150 {
        return Box::new(MBC1::new(rom_data, 0, 2, 0));
    }

    let mbc_type = MBCType::from_cartridge_type(rom_data[0x147]);
    let rom_size = get_rom_size_bytes(rom_data[0x148]);
    let ram_size = get_ram_size_bytes(rom_data[0x149]);

    let rom_banks = (rom_size / 0x4000) as u16;
    let ram_banks = if ram_size > 0 {
        (ram_size / 0x2000) as u8
    } else {
        0
    };

    match mbc_type {
        MBCType::None => Box::new(MBC1::new(rom_data, 0, 2, 0)),
        MBCType::MBC1 => Box::new(MBC1::new(rom_data, ram_size, rom_banks, ram_banks)),
        MBCType::MBC2 => Box::new(MBC2::new(rom_data, rom_banks)),
        MBCType::MBC3 => Box::new(MBC3::new(rom_data, ram_size, rom_banks, ram_banks)),
        MBCType::MBC5 => Box::new(MBC5::new(rom_data, ram_size, rom_banks, ram_banks)),
        MBCType::Unknown(_) => Box::new(MBC1::new(rom_data, 0, 2, 0)),
    }
}
