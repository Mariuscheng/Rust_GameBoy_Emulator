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
mod joypad;
mod timer;

fn main() {
    println!("Game Boy 模擬器啟動中..."); // 創建 MMU 和 CPU
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();
    println!("系統組件初始化完成");

    // 載入實際 ROM 檔案
    let rom_path = "rom.gb";
    match std::fs::read(rom_path) {
        Ok(rom_data) => {
            cpu.load_rom(&rom_data);
            println!("成功載入 ROM: {} ({} bytes)", rom_path, rom_data.len());
        }
        Err(e) => {
            // 如果無法載入實際 ROM，使用最小化測試 ROM
            println!("無法載入 ROM '{}': {}", rom_path, e);
            println!("使用內建測試 ROM...");
            let test_rom = vec![
                0x00, // NOP
                0x3E, 0x42, // LD A, 0x42
                0x18, 0xFC, // JR -4 (跳回 NOP)
            ];
            cpu.load_rom(&test_rom);
        }
    }
    // 創建窗口
    let mut window = Window::new("Game Boy 模擬器", 160, 144, WindowOptions::default()).unwrap();
    let mut frame_count = 0;
    let start_time = std::time::Instant::now();
    println!("開始模擬循環...");
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 執行 CPU 模擬步驟
        cpu.step();

        // 從 MMU 獲取 VRAM 數據，但確保不要注入測試數據
        // 直接獲取當前狀態的 VRAM
        let vram_data = cpu.mmu.vram();
        ppu.vram.copy_from_slice(&vram_data);

        // 同步其他 PPU 相關寄存器
        ppu.set_oam(cpu.mmu.oam());
        ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
        ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
        ppu.set_scx(cpu.mmu.read_byte(0xFF43));
        ppu.set_scy(cpu.mmu.read_byte(0xFF42));
        ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A));
        ppu.set_lcdc(cpu.mmu.read_byte(0xFF40)); // 設置 LCD 控制寄存器

        // 執行 PPU 渲染
        ppu.step();

        // 更新窗口顯示
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
                println!("======== 詳細 VRAM 分析 (幀數: {}) ========", frame_count);

                // 測試新的簡單方法
                println!("簡單測試方法結果: {}", cpu.mmu.test_simple_method());
                println!("簡單版本: {}", cpu.mmu.simple_version());

                // 重新啟用詳細 VRAM 分析
                let vram_analysis = cpu.mmu.analyze_vram_content();
                println!("{}", vram_analysis);
                cpu.mmu.save_vram_analysis();
            }

            // 檢查背景 tile map 前幾個字節
            print!("  背景 tile map 前16字節: ");
            for i in 0x1800..0x1810 {
                print!("{:02X} ", ppu.vram[i]);
            }
            println!();

            println!("{}", cpu.get_status_report());

            // 每 50000 幀保存性能報告
            if frame_count % 50000 == 0 {
                cpu.save_performance_report();
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
    println!("{}", cpu.get_status_report());

    // 保存最終性能報告
    cpu.save_performance_report();

    println!("\nGame Boy 模擬器結束");
}
