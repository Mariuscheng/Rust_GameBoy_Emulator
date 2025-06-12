// Game Boy 模擬器 - 主程式
// 完整版本：包含 LCDC 保護機制和完整功能

use minifb::{Key, Window, WindowOptions};

mod apu;
mod cpu;
mod joypad;
mod mmu;
mod ppu;
mod timer;

use crate::apu::APU;
use crate::cpu::CPU;
use crate::joypad::{GameBoyKey, Joypad};
use crate::mmu::MMU;
use crate::ppu::PPU;
use crate::timer::Timer;

fn main() {
    println!("🎮 Game Boy 模擬器啟動中...");

    // 處理命令行參數
    let args: Vec<String> = std::env::args().collect();
    let rom_file = if args.len() > 1 { &args[1] } else { "rom.gb" };

    // 初始化核心組件
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();
    let mut joypad = Joypad::new();
    let _apu = APU::new();
    let _timer = Timer::new();

    println!("✅ 系統組件初始化完成");

    // 載入 ROM
    println!("🔍 正在載入 ROM 文件: {}", rom_file);
    match std::fs::read(rom_file) {
        Ok(rom_data) => {
            println!("✅ ROM 載入成功: {} ({} bytes)", rom_file, rom_data.len());
            cpu.load_rom(&rom_data);

            println!("📦 ROM 標題: {}", cpu.mmu.rom_info.title);
        }
        Err(e) => {
            println!("❌ 無法載入 ROM 文件 '{}': {}", rom_file, e);
            std::process::exit(1);
        }
    }

    // 創建顯示窗口
    println!("🪟 正在創建顯示窗口...");
    let mut window = match Window::new("Game Boy 模擬器", 160, 144, WindowOptions::default()) {
        Ok(win) => {
            println!("✅ 窗口創建成功");
            win
        }
        Err(e) => {
            println!("❌ 窗口創建失敗: {:?}", e);
            std::process::exit(1);
        }
    };

    // 初始化顯示設置
    let initial_lcdc = 0x91; // LCD 和背景顯示開啟
    cpu.mmu.write_byte(0xFF40, initial_lcdc);
    ppu.set_lcdc(initial_lcdc);

    // 設置標準調色板
    let standard_palette = 0xE4;
    cpu.mmu.write_byte(0xFF47, standard_palette); // BGP
    cpu.mmu.write_byte(0xFF48, standard_palette); // OBP0
    cpu.mmu.write_byte(0xFF49, standard_palette); // OBP1

    // 主循環
    let mut frame_count = 0;
    let mut last_time = std::time::Instant::now();
    println!("🚀 開始模擬循環...");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // 處理輸入
        handle_input(&mut window, &mut joypad, &mut cpu);

        // CPU 執行
        for _ in 0..1000 {
            cpu.step();
        }

        // 更新 PPU
        update_ppu(&mut ppu, &mut cpu);

        // 更新顯示
        if window
            .update_with_buffer(ppu.get_framebuffer(), 160, 144)
            .is_ok()
        {
            if frame_count % 60 == 0 {
                let elapsed = last_time.elapsed();
                let fps = 60.0 / elapsed.as_secs_f32();
                last_time = std::time::Instant::now();
                window.set_title(&format!("Game Boy 模擬器 - {:.1} FPS - {}", fps, rom_file));
            }
        }

        frame_count += 1;
    }

    println!("🎉 Game Boy 模擬器結束");
    println!("📊 總幀數: {}", frame_count);
}

fn handle_input(window: &mut Window, joypad: &mut Joypad, cpu: &mut CPU) {
    let mut updated = false;

    // 方向鍵
    if window.is_key_down(Key::Up) {
        joypad.key_down(GameBoyKey::Up);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::Up) {
        joypad.key_up(GameBoyKey::Up);
        updated = true;
    }

    if window.is_key_down(Key::Down) {
        joypad.key_down(GameBoyKey::Down);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::Down) {
        joypad.key_up(GameBoyKey::Down);
        updated = true;
    }

    if window.is_key_down(Key::Left) {
        joypad.key_down(GameBoyKey::Left);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::Left) {
        joypad.key_up(GameBoyKey::Left);
        updated = true;
    }

    if window.is_key_down(Key::Right) {
        joypad.key_down(GameBoyKey::Right);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::Right) {
        joypad.key_up(GameBoyKey::Right);
        updated = true;
    }

    // A/B 按鈕
    if window.is_key_down(Key::Z) {
        joypad.key_down(GameBoyKey::A);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::A) {
        joypad.key_up(GameBoyKey::A);
        updated = true;
    }

    if window.is_key_down(Key::X) {
        joypad.key_down(GameBoyKey::B);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::B) {
        joypad.key_up(GameBoyKey::B);
        updated = true;
    }

    // Start/Select
    if window.is_key_down(Key::Enter) {
        joypad.key_down(GameBoyKey::Start);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::Start) {
        joypad.key_up(GameBoyKey::Start);
        updated = true;
    }

    if window.is_key_down(Key::Space) {
        joypad.key_down(GameBoyKey::Select);
        updated = true;
    } else if joypad.is_key_pressed(&GameBoyKey::Select) {
        joypad.key_up(GameBoyKey::Select);
        updated = true;
    }

    if updated {
        joypad.update();
        cpu.mmu.joypad = joypad.clone();
    }
}

fn update_ppu(ppu: &mut PPU, cpu: &mut CPU) {
    // 同步 VRAM 到 PPU
    let vram_data = cpu.mmu.vram();
    ppu.vram.copy_from_slice(&vram_data);

    // 更新 PPU 狀態
    ppu.set_oam(cpu.mmu.oam());
    ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
    ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
    ppu.set_obp1(cpu.mmu.read_byte(0xFF49));
    ppu.set_scx(cpu.mmu.read_byte(0xFF43));
    ppu.set_scy(cpu.mmu.read_byte(0xFF42));
    ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
    ppu.set_wy(cpu.mmu.read_byte(0xFF4A));
    ppu.set_lcdc(cpu.mmu.read_byte(0xFF40));

    // 執行 PPU 渲染
    ppu.step(&mut cpu.mmu);
}
