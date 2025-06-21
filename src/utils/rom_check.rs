//! Game Boy ROM header inspection tool
use std::fs;

pub fn check_rom_header(path: &str) {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            println!("❌ Failed to read ROM: {}", e);
            return;
        }
    };
    if data.len() < 0x150 {
        println!("❌ ROM file too small: {} bytes", data.len());
        return;
    }
    let title = &data[0x134..0x144];
    let title_str = String::from_utf8_lossy(title);
    let cgb_flag = data[0x143];
    let licensee = data[0x144];
    let sgb_flag = data[0x146];
    let cart_type = data[0x147];
    let rom_size = data[0x148];
    let ram_size = data[0x149];
    let checksum = data[0x14D];
    let global_checksum = ((data[0x14E] as u16) << 8) | data[0x14F] as u16;
    println!("ROM Title: {}", title_str.trim());
    println!("CGB Flag: 0x{:02X}", cgb_flag);
    println!("Licensee: 0x{:02X}", licensee);
    println!("SGB Flag: 0x{:02X}", sgb_flag);
    println!("Cartridge Type: 0x{:02X}", cart_type);
    println!("ROM Size: 0x{:02X}", rom_size);
    println!("RAM Size: 0x{:02X}", ram_size);
    println!("Header Checksum: 0x{:02X}", checksum);
    println!("Global Checksum: 0x{:04X}", global_checksum);
}
