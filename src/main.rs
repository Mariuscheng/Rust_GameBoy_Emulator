// Game Boy 模擬器 - 主程式
// 清理版本：移除調試代碼，實現乾淨的模擬器

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
use crate::joypad::{GameBoyKey, Joypad};
#[path = "tests_backup/test_runner.rs"]
mod test_runner;
mod timer;
use crate::test_runner::run_test_simulation;
use crate::timer::Timer;
#[cfg(test)]
#[path = "tests_backup/opcode_test.rs"]
mod opcode_test;

fn main() {
    println!("Game Boy 模擬器啟動中...");

    // 檢查命令行參數是否為測試模式
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "test" {
        println!("執行測試模式...");
        let test_result = run_test_simulation();
        println!("{}", test_result); // 將測試結果保存到文件
        if let Ok(mut file) = std::fs::File::create("debug_report/test_result.txt") {
            use std::io::Write;
            let _ = file.write_all(test_result.as_bytes());
            println!("測試結果已保存到 debug_report/test_result.txt");
        }
        return;
    } // 正常模式：創建 MMU 和 CPU
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();
    let mut apu = APU::new();
    let mut joypad = Joypad::new();
    let mut timer = Timer::new();

    // 啟用調試模式
    joypad.set_debug_mode(true);
    apu.set_debug_mode(true);

    println!("系統組件初始化完成");

    // 載入實際 ROM 檔案
    let rom_path = "rom.gb";
    match std::fs::read(rom_path) {
        Ok(rom_data) => {
            cpu.load_rom(&rom_data);
            println!("成功載入 ROM: {} ({} bytes)", rom_path, rom_data.len());
        }
        Err(e) => {
            // 如果無法載入實際 ROM，使用改進的測試 ROM
            println!("無法載入 ROM '{}': {}", rom_path, e);
            println!("使用內建測試 ROM...");
            let test_rom = vec![
                0x3E, 0x91, // LD A, 0x91 (確保 LCDC 正確設定)
                0xE0, 0x40, // LDH (0xFF40), A (設定 LCDC)
                0x3E, 0xFC, // LD A, 0xFC (設定背景調色板)
                0xE0, 0x47, // LDH (0xFF47), A (設定 BGP)
                0x3E, 0xFF, // LD A, 0xFF (設定瓦片數據)
                0xEA, 0x00, 0x80, // LD (0x8000), A (寫入 VRAM 瓦片數據)
                0x3E, 0x01, // LD A, 0x01 (設定瓦片 ID)
                0xEA, 0x00, 0x98, // LD (0x9800), A (寫入背景瓦片地圖)
                0x00, // NOP
                0x18, 0xFE, // JR -2 (無限循環)
            ];
            cpu.load_rom(&test_rom);
        }
    } // 創建窗口
    let mut window = Window::new("Game Boy 模擬器", 160, 144, WindowOptions::default()).unwrap();
    let mut frame_count = 0;
    let start_time = std::time::Instant::now();
    let mut cycle_count = 0;
    println!("開始模擬循環...");
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 執行多個 CPU 步驟來模擬更快的時鐘速度
        for _ in 0..1000 {
            cpu.step();
            cycle_count += 4; // 假設每條指令需要4個時鐘週期

            // 更新定時器，檢查中斷
            if timer.step(4) {
                let mut if_reg = cpu.mmu.read_byte(0xFF0F);
                if_reg |= 0x04; // Timer 中斷
                cpu.mmu.write_byte(0xFF0F, if_reg);
            }

            // 步進 APU 和 MMU
            apu.step();
            cpu.mmu.step();
            cpu.mmu.step_apu();

            // 模擬 LCD 掃描線（LY 暫存器）
            // Game Boy LCD 的掃描線週期約為456個時鐘週期
            if cycle_count >= 456 {
                cycle_count = 0;
                let current_ly = cpu.mmu.read_byte(0xFF44);
                let next_ly = if current_ly >= 153 { 0 } else { current_ly + 1 };
                cpu.mmu.write_byte(0xFF44, next_ly);

                // 在 VBlank 期間設置中斷標誌
                if next_ly == 144 {
                    // 進入 VBlank，設置中斷標誌
                    let mut if_reg = cpu.mmu.read_byte(0xFF0F);
                    if_reg |= 0x01; // VBlank 中斷
                    cpu.mmu.write_byte(0xFF0F, if_reg);
                }
            }
        }

        // 處理鍵盤輸入
        if window.is_key_down(Key::Right) {
            joypad.key_down(GameBoyKey::Right);
        }
        if window.is_key_down(Key::Left) {
            joypad.key_down(GameBoyKey::Left);
        }
        if window.is_key_down(Key::Up) {
            joypad.key_down(GameBoyKey::Up);
        }
        if window.is_key_down(Key::Down) {
            joypad.key_down(GameBoyKey::Down);
        }
        if window.is_key_down(Key::Z) {
            joypad.key_down(GameBoyKey::A);
        }
        if window.is_key_down(Key::X) {
            joypad.key_down(GameBoyKey::B);
        }
        if window.is_key_down(Key::Enter) {
            joypad.key_down(GameBoyKey::Start);
        }
        if window.is_key_down(Key::Space) {
            joypad.key_down(GameBoyKey::Select);
        }

        // 檢查按鍵釋放
        if !window.is_key_down(Key::Right) {
            joypad.key_up(GameBoyKey::Right);
        }
        if !window.is_key_down(Key::Left) {
            joypad.key_up(GameBoyKey::Left);
        }
        if !window.is_key_down(Key::Up) {
            joypad.key_up(GameBoyKey::Up);
        }
        if !window.is_key_down(Key::Down) {
            joypad.key_up(GameBoyKey::Down);
        }
        if !window.is_key_down(Key::Z) {
            joypad.key_up(GameBoyKey::A);
        }
        if !window.is_key_down(Key::X) {
            joypad.key_up(GameBoyKey::B);
        }
        if !window.is_key_down(Key::Enter) {
            joypad.key_up(GameBoyKey::Start);
        }
        if !window.is_key_down(Key::Space) {
            joypad.key_up(GameBoyKey::Select);
        } // 將手柄狀態寫入MMU
        cpu.mmu.set_joypad(joypad.get_joypad_state());

        // 檢查手柄中斷
        if joypad.has_key_pressed() {
            let mut if_reg = cpu.mmu.read_byte(0xFF0F);
            if_reg |= 0x10; // Joypad 中斷
            cpu.mmu.write_byte(0xFF0F, if_reg);
        }

        // 模擬硬體狀態更新
        cpu.simulate_hardware_state();

        // 同步 VRAM、OAM、palette、滾動、window到PPU
        let vram_data = cpu.mmu.vram();
        ppu.vram.copy_from_slice(&vram_data);
        ppu.set_oam(cpu.mmu.oam());
        ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
        ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
        ppu.set_scx(cpu.mmu.read_byte(0xFF43));
        ppu.set_scy(cpu.mmu.read_byte(0xFF42));
        ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A));
        ppu.set_lcdc(cpu.mmu.read_byte(0xFF40)); // 設置 LCD 控制寄存器        ppu.step();

        // 每 2000 幀輸出一次 VRAM 調試信息
        if frame_count % 2000 == 0 {
            let vram_non_zero_count = vram_data.iter().filter(|&&b| b != 0).count();
            println!(
                "VRAM 非零字節數: {} / {}",
                vram_non_zero_count,
                vram_data.len()
            );

            // 檢查瓦片地圖區域
            let tilemap_data = &vram_data[0x1800..0x1C00]; // 背景瓦片地圖
            let tilemap_non_zero = tilemap_data.iter().filter(|&&b| b != 0).count();
            println!("背景瓦片地圖非零字節: {} / 1024", tilemap_non_zero);

            // 檢查瓦片數據區域的前 16 個瓦片
            println!("前 16 個瓦片 ID: {:02X?}", &vram_data[0x1800..0x1810]);

            // 檢查調色板
            let bgp = cpu.mmu.read_byte(0xFF47);
            println!("背景調色板 (BGP): 0x{:02X}", bgp);
        }

        window
            .update_with_buffer(ppu.get_framebuffer(), 160, 144)
            .unwrap();
        frame_count += 1; // 每 1000 幀輸出詳細狀態
        if frame_count % 1000 == 0 {
            println!("======== 幀數: {} ========", frame_count);
            let lcdc_value = cpu.mmu.read_byte(0xFF40);
            println!(
                "LCDC 狀態: 0x{:02X} (LCD {})",
                lcdc_value,
                if (lcdc_value & 0x80) != 0 {
                    "啟用"
                } else {
                    "關閉"
                }
            );
            // 詳細分析 LCDC 各個位
            println!("LCDC 位分析:");
            println!(
                "  Bit 7 (LCD 啟用): {}",
                if (lcdc_value & 0x80) != 0 {
                    "是"
                } else {
                    "否"
                }
            );
            println!(
                "  Bit 6 (Window tile map): 0x{:X}000",
                if (lcdc_value & 0x40) != 0 { 0x9C } else { 0x98 }
            );
            println!(
                "  Bit 5 (Window 啟用): {}",
                if (lcdc_value & 0x20) != 0 {
                    "是"
                } else {
                    "否"
                }
            );
            println!(
                "  Bit 4 (BG & Window tile data): 0x{:X}000",
                if (lcdc_value & 0x10) != 0 { 0x80 } else { 0x90 }
            );
            println!(
                "  Bit 3 (BG tile map): 0x{:X}00",
                if (lcdc_value & 0x08) != 0 { 0x9C } else { 0x98 }
            );
            println!(
                "  Bit 2 (Sprite 大小): {}x{}",
                8,
                if (lcdc_value & 0x04) != 0 { 16 } else { 8 }
            );
            println!(
                "  Bit 1 (Sprite 啟用): {}",
                if (lcdc_value & 0x02) != 0 {
                    "是"
                } else {
                    "否"
                }
            );
            println!(
                "  Bit 0 (背景啟用): {}",
                if (lcdc_value & 0x01) != 0 {
                    "是"
                } else {
                    "否"
                }
            ); // 顯示其他 PPU 寄存器
            println!("其他 PPU 寄存器:");
            println!("  BGP (背景調色板): 0x{:02X}", cpu.mmu.read_byte(0xFF47));
            println!("  SCX (背景滾動X): {}", cpu.mmu.read_byte(0xFF43));
            println!("  SCY (背景滾動Y): {}", cpu.mmu.read_byte(0xFF42)); // 檢查 VRAM 前幾個字節是否有數據
            println!("VRAM 內容檢查:");
            print!("  前16字節: ");
            for i in 0..16 {
                print!("{:02X} ", ppu.vram[i]);
            }
            println!(); // 每 10000 幀進行一次詳細的 VRAM 分析
            if frame_count % 10000 == 0 {
                println!("======== 詳細 VRAM 分析 (幀數: {}) ========", frame_count); // 測試新的簡單方法
                println!("簡單測試方法結果: {}", cpu.mmu.test_simple_method());
                println!("簡單版本: {}", cpu.mmu.simple_version());
                println!("MMU 版本: {}", cpu.mmu.get_mmu_version());
                println!("測試方法結果: {}", cpu.mmu.test_method()); // 顯示MMU調試字段信息
                cpu.mmu.debug_fields();

                // 測試 VRAM 讀寫功能
                let test_vram_value = cpu.mmu.read_vram(0x8000);
                cpu.mmu.write_vram(0x8000, test_vram_value.wrapping_add(1));
                println!("VRAM 測試: 讀取 0x8000 = 0x{:02X}", test_vram_value);

                // 獲取 APU 實例進行額外測試
                let _apu_ref = cpu.mmu.get_apu();

                // 重新啟用詳細 VRAM 分析
                let vram_analysis = cpu.mmu.analyze_vram_content();
                println!("{}", vram_analysis);
                cpu.mmu.save_vram_analysis();

                // 生成並顯示手柄狀態報告
                println!("{}", joypad.generate_status_report());

                // 生成並顯示APU狀態報告
                println!("{}", apu.generate_status_report());
            }

            // 檢查背景 tile map 前幾個字節
            print!("  背景 tile map 前16字節: ");
            for i in 0x1800..0x1810 {
                print!("{:02X} ", ppu.vram[i]);
            }
            println!();

            println!("{}", cpu.get_enhanced_status_report());

            // 檢查是否在等待循環中
            if cpu.is_in_wait_loop() {
                println!("檢測到等待循環 - 這是正常的Game Boy行為");
            } // 每 50000 幀保存性能報告
            if frame_count % 50000 == 0 {
                cpu.save_performance_report();

                // 重置手柄狀態（模擬長時間運行後的狀態重置）
                joypad.reset();
                println!("手柄狀態已重置");
            }
        }
    }

    // 輸出最終統計和保存報告
    let total_time = start_time.elapsed();
    let avg_fps = frame_count as f64 / total_time.as_secs_f64();
    let instruction_count = cpu.get_instruction_count();

    println!("\n================================================================================");
    println!("Game Boy 模擬器執行完畢");
    println!("================================================================================");
    println!(
        "模擬器統計 - 總幀數: {}, 平均 FPS: {:.2}, 總運行時間: {:.2}s",
        frame_count,
        avg_fps,
        total_time.as_secs_f64()
    );
    println!(
        "性能統計 - 總指令數: {}, 平均每秒指令數: {:.0}",
        instruction_count,
        instruction_count as f64 / total_time.as_secs_f64()
    );
    println!("\n最終 CPU 狀態:");
    println!("{}", cpu.get_enhanced_status_report()); // 保存最終性能報告
    cpu.save_performance_report();

    // 保存最終 APU 和手柄報告
    apu.save_final_report();
    joypad.save_final_report();

    println!("\nGame Boy 模擬器結束");
}
