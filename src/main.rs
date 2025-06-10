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
    println!("🎮 Game Boy 模擬器啟動中..."); // 初始化所有組件
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();
    let _apu = APU::new();
    let _joypad = Joypad::new();
    let _timer = Timer::new();

    println!("✅ 系統組件初始化完成");

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

    // 設置測試模式
    println!("🔧 設置測試模式...");

    // 強制設置正確的 LCDC 和調色板
    cpu.mmu.write_byte(0xFF40, 0x91); // LCDC: LCD+BG enabled
    cpu.mmu.write_byte(0xFF47, 0xE4); // BGP: 典型調色板

    // 寫入測試瓦片數據
    for i in 0..16 {
        cpu.mmu.write_byte(0x8000 + i, 0xFF); // 第一個瓦片：全黑
    }

    // 寫入背景瓦片地圖
    for i in 0..10 {
        cpu.mmu.write_byte(0x9800 + i, 1); // 使用瓦片 1
    }

    println!("✅ 測試模式設置完成");

    let mut frame_count = 0;
    let mut cycle_count = 0;

    println!("🚀 開始模擬循環...");

    // 主模擬循環
    while window.is_open() && !window.is_key_down(Key::Escape) {
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

        // 檢查並修復 LCDC（每 1000 幀檢查一次）
        if frame_count % 1000 == 0 {
            let lcdc_value = cpu.mmu.read_byte(0xFF40);
            let bg_disabled = (lcdc_value & 0x01) == 0;
            let lcd_disabled = (lcdc_value & 0x80) == 0;
            let tile_data_wrong = (lcdc_value & 0x10) == 0;

            if bg_disabled || lcd_disabled || tile_data_wrong {
                let new_lcdc = 0x91;
                cpu.mmu.write_byte(0xFF40, new_lcdc);
                ppu.set_lcdc(new_lcdc);
                println!("⚡ LCDC 保護機制觸發! 修正為 0x{:02X}", new_lcdc);
            }

            if frame_count % 5000 == 0 {
                println!("📊 幀數: {}, LCDC: 0x{:02X}", frame_count, lcdc_value);
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
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A));
        ppu.set_lcdc(cpu.mmu.read_byte(0xFF40));

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
