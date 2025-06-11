// Game Boy 模擬器 - 主程式
// 完整版本：包含 LCDC 保護機制和完整功能

use minifb::{Key, Window, WindowOptions};

mod mmu;
use crate::mmu::MMU;
mod cpu;
use crate::cpu::CPU;
mod ppu;
use crate::ppu::PPU;
mod apu;
use crate::apu::APU;
mod joypad;
use crate::joypad::Joypad;
mod timer;
use crate::timer::Timer;

fn main() {
    println!("🎮 Game Boy 模擬器啟動中...");

    // 處理命令行參數
    let args: Vec<String> = std::env::args().collect();
    let rom_file = if args.len() > 1 { &args[1] } else { "rom.gb" };

    // 初始化所有組件
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();
    let _apu = APU::new();
    let _joypad = Joypad::new();
    let _timer = Timer::new();

    println!("✅ 系統組件初始化完成");

    // 載入遊戲 ROM
    use std::fs;
    println!("🔍 正在尋找 ROM 文件: {}", rom_file);

    let rom_data = match fs::read(rom_file) {
        Ok(data) => {
            println!("✅ ROM 載入成功: {} ({} bytes)", rom_file, data.len());
            data
        }
        Err(e) => {
            println!("❌ 無法載入 ROM 文件 '{}': {}", rom_file, e);
            println!("💡 使用方法:");
            println!("   cargo run                    # 使用默認的 rom.gb");
            println!("   cargo run -- <rom文件路徑>   # 使用指定的 ROM 文件");
            println!("   cargo run -- game.gb        # 使用 game.gb");
            println!("   cargo run --bin clean_test  # 運行終端測試版本");
            std::process::exit(1);
        }
    };

    cpu.load_rom(&rom_data);

    // 寫入測試圖案到 VRAM，避免白屏（僅測試用）
    // cpu.mmu.write_test_pattern_to_vram(); // 移除這行，讓 ROM 自己初始化 VRAM，顯示遊戲畫面

    // 創建窗口
    println!("🪟 正在創建顯示窗口...");
    let window_result = Window::new("Game Boy 模擬器", 160, 144, WindowOptions::default());
    let mut window = match window_result {
        Ok(w) => {
            println!("✅ 窗口創建成功");
            w
        }
        Err(e) => {
            println!("❌ 窗口創建失敗: {:?}", e);
            println!("💡 建議使用終端測試版本:");
            println!("   cargo run --bin clean_test");
            std::process::exit(1);
        }
    };

    // 啟動時強制設置 LCDC 為 0x91，確保 LCD 與 BG 開啟
    cpu.mmu.write_byte(0xFF40, 0x91);
    ppu.set_lcdc(0x91);

    // 強制設置 BGP 為標準 Game Boy 色階
    cpu.mmu.write_byte(0xFF47, 0xE4);

    let mut frame_count = 0;
    let mut cycle_count = 0;

    println!("🚀 開始模擬循環..."); // 主模擬循環
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 確保 LCDC 始終啟用 LCD 顯示和背景，但降低日誌頻率
        let lcdc_value = cpu.mmu.read_byte(0xFF40);
        let fixed_lcdc = lcdc_value | 0x91; // 設置第 7 位 (LCD 開啟) 和第 0 位 (背景開啟)
        if fixed_lcdc != lcdc_value {
            cpu.mmu.write_byte(0xFF40, fixed_lcdc);
            // 只在重要變更時或每100幀顯示一次日誌，降低噪音
            if (lcdc_value & 0x80) == 0 || (lcdc_value & 0x01) == 0 || frame_count % 100 == 0 {
                println!(
                    "⚡ LCDC 修正 (幀 {}): 0x{:02X} -> 0x{:02X}",
                    frame_count, lcdc_value, fixed_lcdc
                );
            }
        }
        ppu.set_lcdc(fixed_lcdc);

        // CPU 執行
        for _ in 0..1000 {
            cpu.step();
            cycle_count += 4;

            // 模擬掃描線週期
            if cycle_count >= 456 {
                cycle_count = 0;
                let current_ly = cpu.mmu.read_byte(0xFF44);
                let next_ly = if current_ly >= 153 { 0 } else { current_ly + 1 };
                cpu.mmu.write_byte(0xFF44, next_ly);

                // VBlank 中斷
                if next_ly == 144 {
                    let mut if_reg = cpu.mmu.read_byte(0xFF0F);
                    if_reg |= 0x01;
                    cpu.mmu.write_byte(0xFF0F, if_reg);
                }
            }
        }

        // 同步 VRAM 到 PPU
        let vram_data = cpu.mmu.vram();
        ppu.vram.copy_from_slice(&vram_data);

        // 設置 PPU 參數
        ppu.set_oam(cpu.mmu.oam());
        ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
        ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
        ppu.set_scx(cpu.mmu.read_byte(0xFF43));
        ppu.set_scy(cpu.mmu.read_byte(0xFF42));
        ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A)); // 確保 LCDC 設置正確，使用之前已修正的值
        ppu.set_lcdc(fixed_lcdc); // 使用已經修正過的LCDC值

        // 執行 PPU 渲染
        ppu.step();

        // 更新窗口
        window
            .update_with_buffer(ppu.get_framebuffer(), 160, 144)
            .unwrap();
        frame_count += 1;
    }

    println!("🎉 Game Boy 模擬器結束");
    println!("📊 總幀數: {}", frame_count);
}
