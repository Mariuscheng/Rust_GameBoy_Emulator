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
        cpu.step();
        // 同步 VRAM、OAM、palette、滾動、window
        ppu.vram.copy_from_slice(&cpu.mmu.vram());
        ppu.set_oam(cpu.mmu.oam().try_into().unwrap());
        ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
        ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
        ppu.set_scx(cpu.mmu.read_byte(0xFF43));
        ppu.set_scy(cpu.mmu.read_byte(0xFF42));
        ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A));
        ppu.set_lcdc(cpu.mmu.read_byte(0xFF40)); // 設置 LCD 控制寄存器
        ppu.step();
        window
            .update_with_buffer(ppu.get_framebuffer(), 160, 144)
            .unwrap();
        frame_count += 1; // 每 10000 幀輸出詳細狀態
        if frame_count % 10000 == 0 {
            println!("======== 幀數: {} ========", frame_count);
            println!(
                "LCDC 狀態: 0x{:02X} (LCD {})",
                cpu.mmu.read_byte(0xFF40),
                if (cpu.mmu.read_byte(0xFF40) & 0x80) != 0 {
                    "啟用"
                } else {
                    "關閉"
                }
            );
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
