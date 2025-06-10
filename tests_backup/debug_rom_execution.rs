// ROM執行調試工具
use std::fs;

fn main() {
    println!("=== Game Boy ROM執行調試工具 ===");

    // 檢查ROM文件
    let rom_path = "rom.gb";
    match fs::read(rom_path) {
        Ok(rom_data) => {
            println!("✓ ROM文件載入成功: {} bytes", rom_data.len()); // 分析ROM標頭
            if rom_data.len() >= 0x150 {
                let title_bytes = &rom_data[0x134..0x144];
                let title_string = String::from_utf8_lossy(title_bytes);
                let title = title_string.trim_end_matches('\0');
                println!("  遊戲標題: {}", title);

                let cartridge_type = rom_data[0x147];
                println!("  卡帶類型: 0x{:02X}", cartridge_type);

                let rom_size = rom_data[0x148];
                println!("  ROM大小: 0x{:02X}", rom_size);

                let ram_size = rom_data[0x149];
                println!("  RAM大小: 0x{:02X}", ram_size);

                // 檢查入口點的指令
                if rom_data.len() > 0x100 {
                    println!("  入口點(0x0100)的前8字節:");
                    print!("    ");
                    for i in 0x100..0x108 {
                        print!("{:02X} ", rom_data[i]);
                    }
                    println!();

                    // 分析第一條指令
                    let first_opcode = rom_data[0x100];
                    println!("  第一條指令: 0x{:02X}", first_opcode);

                    match first_opcode {
                        0x00 => println!("    -> NOP (無操作)"),
                        0x3E => println!("    -> LD A,n (載入立即數到A)"),
                        0xC3 => println!("    -> JP nn (跳轉到地址)"),
                        0x18 => println!("    -> JR n (相對跳轉)"),
                        _ => println!("    -> 未知指令或其他指令"),
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ 無法載入ROM文件: {}", e);
            return;
        }
    }

    println!("\n=== 建議的調試步驟 ===");
    println!("1. 確認ROM正確載入到MMU");
    println!("2. 檢查CPU的PC是否設置為0x0100");
    println!("3. 執行第一條指令並檢查結果");
    println!("4. 監控VRAM的寫入操作");
    println!("5. 檢查PPU的背景瓦片映射");
}
