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

    // 檢查並顯示 ROM 標題
    if let Some(title) = cpu.mmu.get_rom_title() {
        println!("📦 ROM 標題: {}", title);
    } else {
        println!("⚠️ 未能讀取 ROM 標題");
    }

    // 驗證 ROM 完整性
    if let Some(checksum) = cpu.mmu.verify_rom_integrity() {
        println!("📊 ROM 校驗和: {}", checksum);
    }

    // 顯示 VRAM 分析
    println!("🧩 {}", cpu.mmu.analyze_vram_content());

    // 新增 VRAM 詳細分析
    println!("🔍 VRAM 詳細分析:");
    let vram_data = cpu.mmu.vram();
    let non_zero_count = vram_data.iter().filter(|&&b| b != 0).count();
    println!(
        "  - 非零字節: {} / {} 字節",
        non_zero_count,
        vram_data.len()
    );

    // 顯示前 256 個字節的樣本
    if non_zero_count > 0 {
        println!("  - VRAM 前 16 個字節樣本:");
        for i in 0..16 {
            if i < vram_data.len() && vram_data[i] != 0 {
                println!("    位置 0x{:04X}: 0x{:02X}", i, vram_data[i]);
            }
        }
    }

    // 讓系統執行一段時間以啟動 ROM 初始化例程
    println!("🔄 執行 ROM 初始化例程...");
    for i in 0..500000 {
        cpu.step();

        if i % 100000 == 0 {
            println!("💾 初始化進度: {} 指令", i);
            // 檢查 VRAM 狀態
            let vram_usage = cpu.mmu.vram().iter().filter(|&&b| b != 0).count();
            if vram_usage > 0 {
                println!("🧩 VRAM 已開始載入: {} 字節非零", vram_usage);
            }
        }
    }
    println!("✅ 初始化過程完成");

    // 檢查 Tetris ROM 是否正確載入了 VRAM 數據
    let vram_data = cpu.mmu.vram();
    let non_zero_count = vram_data.iter().filter(|&&b| b != 0).count();
    println!(
        "🎮 Tetris VRAM 數據檢查: {} / {} 字節非零",
        non_zero_count,
        vram_data.len()
    );

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
    }; // 設置 LCDC 寄存器初始值
       // 0x91 (10010001):
       // - Bit 7: LCD 顯示開啟 (1)
       // - Bit 4: BG & Window Tile Data ($8000-$8FFF) (1)
       // - Bit 0: BG & Window 顯示開啟 (1)
    let initial_lcdc = 0x91;
    cpu.mmu.write_byte(0xFF40, initial_lcdc);
    ppu.set_lcdc(initial_lcdc); // 設置 BGP 為標準 Game Boy 調色板
                                // 0xE4 (11100100) = %11 %10 %01 %00 的顏色值順序，即：
                                // - 顏色 3 = 黑 (11)
                                // - 顏色 2 = 深灰 (10)
                                // - 顏色 1 = 淺灰 (01)
                                // - 顏色 0 = 白 (00)
    let standard_palette = 0xE4;
    cpu.mmu.write_byte(0xFF47, standard_palette);

    // 確保所有其他顯示相關寄存器被設置
    cpu.mmu.write_byte(0xFF48, standard_palette); // OBP0
    cpu.mmu.write_byte(0xFF49, standard_palette); // OBP1

    let mut frame_count = 0;
    let mut cycle_count = 0;

    println!("🚀 開始模擬循環..."); // 主模擬循環
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 確保 LCDC 設定正確，僅保證 LCD 顯示始終啟用
        let lcdc_value = cpu.mmu.read_byte(0xFF40); // 優化的 LCDC 保護策略：
                                                    // 1. 確保 LCD 顯示始終開啟 (位元 7)
                                                    // 2. 確保背景顯示始終開啟 (位元 0)
                                                    // 3. 其餘位元保留 ROM 的原始設置，允許遊戲靈活控制顯示功能
        let fixed_lcdc = lcdc_value | 0x81; // 強制開啟 LCD 顯示和背景顯示

        if fixed_lcdc != lcdc_value {
            cpu.mmu.write_byte(0xFF40, fixed_lcdc); // 輸出更詳細的日誌
            let lcd_changed = (lcdc_value & 0x80) == 0;
            let bg_changed = (lcdc_value & 0x01) == 0;
            if lcd_changed || bg_changed {
                println!(
                    "⚡ LCDC 修正 (幀 {}): 顯示設置被調整 (0x{:02X} -> 0x{:02X})",
                    frame_count, lcdc_value, fixed_lcdc
                );
                if lcd_changed {
                    println!("  - LCD 顯示被強制開啟");
                }
                if bg_changed {
                    println!("  - 背景顯示被強制開啟");
                }
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

        // 設置 PPU 參數        ppu.set_oam(cpu.mmu.oam());
        ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
        ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
        ppu.set_obp1(cpu.mmu.read_byte(0xFF49)); // 設置 OBP1 調色板
        ppu.set_scx(cpu.mmu.read_byte(0xFF43));
        ppu.set_scy(cpu.mmu.read_byte(0xFF42));
        ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A)); // 確保 LCDC 設置正確，使用之前已修正的值
        ppu.set_lcdc(fixed_lcdc); // 使用已經修正過的LCDC值

        // 執行 PPU 渲染
        ppu.step(); // 獲取並顯示 FPS
        let fps = ppu.get_fps();
        if fps > 0 {
            let title = format!("Game Boy 模擬器 - {} FPS - {}", fps, rom_file);
            window.set_title(&title);
        }

        // 更新窗口
        window
            .update_with_buffer(ppu.get_framebuffer(), 160, 144)
            .unwrap(); // 輸出 PPU 調試信息
        let debug_info = ppu.debug_info(frame_count);
        if !debug_info.is_empty() {
            println!("{}", debug_info);

            // 每 200 幀檢查 VRAM 狀態（僅用於調試，不干預）
            if frame_count % 200 == 0 {
                let vram_data = cpu.mmu.vram();
                let non_zero_count = vram_data.iter().filter(|&&b| b != 0).count();
                println!(
                    "🎮 VRAM 狀態: {} / {} 字節非零",
                    non_zero_count,
                    vram_data.len()
                );
            }
        }

        // 每幀強制設置調色板為標準值，避免遊戲將其設為 0
        let current_bgp = cpu.mmu.read_byte(0xFF47);
        if current_bgp == 0 {
            cpu.mmu.write_byte(0xFF47, standard_palette); // 重置為標準調色板
            ppu.set_bgp(standard_palette);
            println!("🎨 檢測到調色板被重置為0，已恢復為標準值 (0xE4)");
        }

        frame_count += 1;
    }

    println!("🎉 Game Boy 模擬器結束");
    println!("📊 總幀數: {}", frame_count);
}
