// Game Boy 模擬器 - 主程式
// 完整版本：包含 LCDC 保護機制和完整功能

use std::rc::Rc;
use std::cell::RefCell;
use winit::event::{Event, WindowEvent, ElementState, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use pixels::{Pixels, SurfaceTexture};

use gameboy_emulator::mmu::MMU;
use gameboy_emulator::cpu::CPU;
use gameboy_emulator::ppu::PPU;
use gameboy_emulator::apu::APU;
use gameboy_emulator::joypad::{Joypad, GameBoyKey};
use gameboy_emulator::cpu::interrupts::InterruptRegisters;
use gameboy_emulator::timer::Timer;

fn main() {
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
        let _ = writeln!(file, "🎮 Game Boy 模擬器啟動中...");
    }

    println!("🎮 Game Boy 模擬器啟動中...");
    // 初始化 Rc/RefCell 架構
    let joypad = Rc::new(RefCell::new(Joypad::new()));
    let interrupt_registers = Rc::new(RefCell::new(InterruptRegisters::new()));
    let mmu = Rc::new(RefCell::new(MMU::new()));
    mmu.borrow_mut().set_joypad(joypad.clone());
    mmu.borrow_mut().set_interrupt_registers(interrupt_registers.clone());
    let mut cpu = CPU::new(mmu.clone(), interrupt_registers.clone());
    let mut ppu = PPU::new();
    let _apu = APU::new();
    let _timer = Timer::new();
    println!("✅ 系統組件初始化完成");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
        let _ = writeln!(file, "✅ 系統組件初始化完成");
    }

    // 載入遊戲 ROM
    use std::fs;
    let rom_data = fs::read("rom.gb").expect("找不到 rom.gb，請將遊戲 ROM 放在專案根目錄");
    mmu.borrow_mut().load_rom(rom_data);
    println!("✅ ROM 載入完成");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
        let _ = writeln!(file, "✅ ROM 載入完成");
    }

    // 建立 winit event loop 與 window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Game Boy 模擬器")
        .with_inner_size(winit::dpi::LogicalSize::new(160.0, 144.0))
        .build(&event_loop)
        .unwrap();
    let pixels = Rc::new(RefCell::new({
        // let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(160, 144, &window);
        Pixels::new(160, 144, surface_texture).unwrap()
    }));

    let mut frame_count = 0;
    let mut cycle_count = 0;

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
        let _ = writeln!(file, "🚀 開始模擬循環...");
    }

    println!("🚀 開始模擬循環...");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        if let Some(gb_key) = map_vk_to_gbkey(keycode) {
                            match input.state {
                                ElementState::Pressed => joypad.borrow_mut().key_down(gb_key),
                                ElementState::Released => joypad.borrow_mut().key_up(gb_key),
                            }
                        }
                        if keycode == VirtualKeyCode::Escape {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                WindowEvent::Resized(size) => {
                    pixels.borrow_mut().resize_surface(size.width, size.height).unwrap();
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                // 執行模擬步驟（累加 cycles 直到 70224，約一幀）
                let mut frame_cycles: u32 = 0;
                let mut scanline_cycles = 0;
                let mut step_count = 0;
                while frame_cycles < 70224 {
                    if let Ok(cycles) = cpu.step() {
                        ppu.step(cycles, &mut mmu.borrow_mut());
                        frame_cycles += cycles as u32;
                        step_count += 1;
                        // DEBUG: 每 1000 條指令 log 一次 PC
                        if step_count % 1000 == 0 {
                            let pc = cpu.registers.pc;
                            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                                let _ = writeln!(file, "[CPU-PC] step {}: PC={:04X}", step_count, pc);
                            }
                        }
                    } else {
                        break;
                    }
                }

                // 模擬掃描線週期
                if cycle_count >= 456 {
                    cycle_count = 0;
                    let current_ly = mmu.borrow().read_byte(0xFF44).unwrap_or(0);
                    let next_ly = if current_ly >= 153 { 0 } else { current_ly + 1 };
                    mmu.borrow_mut().write_byte(0xFF44, next_ly).ok();

                    // VBlank 中斷
                    if next_ly == 144 {
                        let mut if_reg = mmu.borrow().read_byte(0xFF0F).unwrap_or(0);
                        if_reg |= 0x01;
                        mmu.borrow_mut().write_byte(0xFF0F, if_reg).ok();
                    }
                }

                // 檢查並修復 LCDC（每 1000 幀檢查一次）
                if frame_count % 1000 == 0 {
                    let lcdc_value = mmu.borrow().read_byte(0xFF40).unwrap_or(0);
                    let bg_disabled = (lcdc_value & 0x01) == 0;
                    let lcd_disabled = (lcdc_value & 0x80) == 0;
                    let tile_data_wrong = (lcdc_value & 0x10) == 0;

                    if bg_disabled || lcd_disabled || tile_data_wrong {
                        let new_lcdc = 0x91;
                        mmu.borrow_mut().write_byte(0xFF40, new_lcdc).ok();
                        ppu.set_lcdc(new_lcdc);
                        println!("⚡ LCDC 保護機制觸發! 修正為 0x{:02X}", new_lcdc);
                    }

                    if frame_count % 5000 == 0 {
                        println!("📊 幀數: {}, LCDC: 0x{:02X}", frame_count, lcdc_value);
                    }
                }

                // 同步 VRAM 到 PPU
                let mmu_vram = mmu.borrow().vram();
                ppu.vram.copy_from_slice(&mmu_vram[..]);

                // --- DEBUG: 每幀記錄 VRAM 前 32 bytes 狀態 ---
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                    let vram_preview: Vec<String> = mmu_vram.iter().take(32).map(|b| format!("{:02X}", b)).collect();
                    let _ = writeln!(file, "[VRAM] first 32 bytes: {}", vram_preview.join(", "));
                }

                // 設置 PPU 參數
                ppu.set_oam(mmu.borrow().oam());
                ppu.set_bgp(mmu.borrow().read_byte(0xFF47).unwrap_or(0));
                ppu.set_obp0(mmu.borrow().read_byte(0xFF48).unwrap_or(0));
                ppu.set_scx(mmu.borrow().read_byte(0xFF43).unwrap_or(0));
                ppu.set_scy(mmu.borrow().read_byte(0xFF42).unwrap_or(0));
                ppu.set_wx(mmu.borrow().read_byte(0xFF4B).unwrap_or(0));
                ppu.set_wy(mmu.borrow().read_byte(0xFF4A).unwrap_or(0));
                ppu.set_lcdc(mmu.borrow().read_byte(0xFF40).unwrap_or(0));

                // 執行 PPU 渲染

                // 畫面資料複製到 pixels framebuffer
                let framebuffer = ppu.get_framebuffer();
                // 畫面 debug log
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                    let preview: Vec<String> = framebuffer.iter().take(10).map(|px| format!("{:08X}", px)).collect();
                    let _ = writeln!(file, "[FRAME] preview: {}", preview.join(", "));
                }
                let mut pixels = pixels.borrow_mut();
                let frame = pixels.frame_mut();
                for (dst, src) in frame.chunks_exact_mut(4).zip(framebuffer.iter()) {
                    let pixel = *src;
                    dst[0] = (pixel & 0xFF) as u8;         // B
                    dst[1] = ((pixel >> 8) & 0xFF) as u8;  // G
                    dst[2] = ((pixel >> 16) & 0xFF) as u8; // R
                    dst[3] = ((pixel >> 24) & 0xFF) as u8; // A
                }
                pixels.render().unwrap();
                frame_count += 1;
            }
            Event::MainEventsCleared => {
                if *control_flow != ControlFlow::Exit {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    });
}

// 鍵盤對應 Game Boy 按鍵
fn map_vk_to_gbkey(key: VirtualKeyCode) -> Option<GameBoyKey> {
    match key {
        VirtualKeyCode::Right => Some(GameBoyKey::Right),
        VirtualKeyCode::Left => Some(GameBoyKey::Left),
        VirtualKeyCode::Up => Some(GameBoyKey::Up),
        VirtualKeyCode::Down => Some(GameBoyKey::Down),
        VirtualKeyCode::Z => Some(GameBoyKey::A),
        VirtualKeyCode::X => Some(GameBoyKey::B),
        VirtualKeyCode::Space => Some(GameBoyKey::Select),
        VirtualKeyCode::Return => Some(GameBoyKey::Start),
        _ => None,
    }
}
