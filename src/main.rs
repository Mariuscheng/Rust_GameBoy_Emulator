// Game Boy 模擬器主程式
use minifb::{Key, Window, WindowOptions};
use std::error::Error;
use std::fs;

// 模組聲明
mod cpu;
mod interrupts;
mod libs;
mod mmu;
mod ppu;
mod sound;
mod timer;

// 導入核心組件
use cpu::CPU;
use libs::cartridge::CartridgeHeader;
use mmu::MMU;
use ppu::PPU;
use timer::Timer;

// 系統常量
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

// 掃描線常量
const SCANLINE_CYCLES: u32 = 456; // 單掃描線的時鐘週期
const IF_REGISTER: u16 = 0xFF0F;
const INPUT_REGISTER: u16 = 0xFF00;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Game Boy 模擬器啟動中..."); // 載入遊戲 ROM
    println!("🎮 選擇遊戲...");
    let rom_path = "rom/dmg_test_prog_ver1.gb"; // 選擇俄羅斯方塊作為預設遊戲
    println!("💾 載入 ROM 檔案: {}", rom_path);

    let rom_data = fs::read(rom_path).map_err(|e| {
        eprintln!("❌ ROM 檔案載入失敗: {:?}", e);
        eprintln!("💡 提示: 請確保遊戲 ROM 檔案位於 rom 目錄中");
        e
    })?;
    println!("✅ ROM 檔案載入完成 ({} bytes)", rom_data.len());

    // 解析 ROM 頭部
    let header = CartridgeHeader::from_rom(&rom_data).ok_or("ROM 頭部解析失敗")?;
    // 遊戲標題已在上面輸出，不需保存到變量

    // 輸出 ROM 資訊
    println!("📝 ROM 資訊:");
    println!("   遊戲標題: {}", header.title);
    println!("   卡帶類型: {:?}", header.cartridge_type);
    println!(
        "   ROM 大小: {:?} ({} KB)",
        header.rom_size,
        header.get_rom_size_in_bytes() / 1024
    );
    println!(
        "   RAM 大小: {:?} ({} KB)",
        header.ram_size,
        header.get_ram_size_in_bytes() / 1024
    );

    // 驗證 Nintendo Logo
    if !header.validate_nintendo_logo() {
        eprintln!("⚠️ 警告：Nintendo Logo 驗證失敗！");
        // return Err("Nintendo Logo 驗證失敗".into());
    }

    // 檢查 ROM 大小
    if rom_data.len() < header.get_rom_size_in_bytes() {
        return Err("ROM 檔案大小不符合頭部宣告".into());
    } // 初始化系統組件
    println!("⚙️ 初始化系統組件...");
    let mmu = MMU::new(rom_data);
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();
    let mut timer = Timer::new();

    // 設置初始硬體狀態
    cpu.mmu.write_byte(0xFF40, 0x91); // LCDC - 啟用 LCD 和背景
    cpu.mmu.write_byte(0xFF42, 0x00); // SCY - 初始垂直捲動
    cpu.mmu.write_byte(0xFF43, 0x00); // SCX - 初始水平捲動
    cpu.mmu.write_byte(0xFF47, 0xFC); // BGP - 背景調色板    // 初始化 PPU 寄存器
    ppu.lcdc = cpu.mmu.read_byte(0xFF40);
    ppu.bgp = cpu.mmu.read_byte(0xFF47);
    ppu.scy = cpu.mmu.read_byte(0xFF42);
    ppu.scx = cpu.mmu.read_byte(0xFF43);

    // 初始化完成後立即輸出 PPU 狀態
    println!("PPU 初始狀態:");
    println!("  LCDC = {:02X}h", ppu.lcdc);
    println!("  BGP  = {:02X}h", ppu.bgp);
    println!("  SCY  = {:02X}h", ppu.scy);
    println!("  SCX  = {:02X}h", ppu.scx);

    println!("✅ 系統組件初始化完成");

    // 創建顯示窗口
    println!("🪟 正在創建顯示窗口...");
    let mut window = Window::new(
        "Game Boy 模擬器",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::X2,
            borderless: false,
            title: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| {
        eprintln!("❌ 窗口創建失敗: {:?}", e);
        eprintln!("💡 提示: 請確保系統支援圖形顯示");
        e
    })?;

    // 設置更新率限制與視窗標題
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600))); // ~60 FPS
    window.set_title(
        "Game Boy 模擬器 - 按 ESC 退出，方向鍵移動，Z:A鍵 X:B鍵 SPACE:Select ENTER:Start",
    ); // 初始化畫面緩衝區和計數器
    let mut frame_buffer = vec![0u32; SCREEN_WIDTH * SCREEN_HEIGHT];
    let mut scanline_cycles: u32 = 0;
    let mut frames: u32 = 0;

    // 設置基本的時間控制變數
    let target_frame_time = std::time::Duration::from_micros(16667); // ~60 FPS
    let mut last_frame_time = std::time::Instant::now();
    let mut accumulated_time = std::time::Duration::from_secs(0);

    println!("🚀 開始模擬循環...");
    println!("🎮 操作方式:");
    println!("   方向鍵: 移動");
    println!("   Z: A鍵  X: B鍵");
    println!("   SPACE: Select");
    println!("   ENTER: Start");
    println!("   ESC: 退出"); // 主模擬循環
    while window.is_open() && !window.is_key_down(Key::Q) && !window.is_key_down(Key::Escape) {
        let current_time = std::time::Instant::now();
        let frame_time = current_time.duration_since(last_frame_time);
        last_frame_time = current_time;
        accumulated_time += frame_time;

        // 更新輸入狀態
        if let Err(e) = update_input(&window, &mut cpu.mmu) {
            eprintln!("⚠️ 輸入更新失敗: {:?}", e);
            continue;
        }

        // 固定時間步進循環
        while accumulated_time >= target_frame_time {
            accumulated_time -= target_frame_time; // CPU 和系統組件更新
            let mut repeat_pc_count = 0;
            let mut last_pc = 0;
            for _ in 0..70 {
                // 檢測死循環
                if cpu.registers.pc == last_pc {
                    repeat_pc_count += 1;
                    if repeat_pc_count > 10 {
                        println!(
                            "⚠️ 可能檢測到死循環 PC=0x{:04X}, 強制繼續",
                            cpu.registers.pc
                        );
                        cpu.registers.pc += 1; // 強制跳過當前指令
                        repeat_pc_count = 0;
                    }
                } else {
                    repeat_pc_count = 0;
                    last_pc = cpu.registers.pc;
                }

                // 每幀執行多個 CPU 週期
                let cycles = cpu.step();
                timer.update(cycles);
                scanline_cycles += cycles as u32;

                // 更新 PPU 狀態
                ppu.lcdc = cpu.mmu.read_byte(0xFF40);
                ppu.scy = cpu.mmu.read_byte(0xFF42);
                ppu.scx = cpu.mmu.read_byte(0xFF43);
                ppu.bgp = cpu.mmu.read_byte(0xFF47);

                // 掃描線更新
                if scanline_cycles >= SCANLINE_CYCLES {
                    scanline_cycles -= SCANLINE_CYCLES; // 每秒輸出一次 PPU 狀態（假設 60fps）
                    if frames % 60 == 0 && ppu.ly == 0 {
                        println!(
                            "\n══ PPU 狀態更新 [幀數: {}] ══\n└─ LCDC={:02X}h BGP={:02X}h SCX={:02X}h SCY={:02X}h",
                            frames, ppu.lcdc, ppu.bgp, ppu.scx, ppu.scy
                        );
                    } // 渲染掃描線並處理 VBlank
                    if ppu.ly < 144 {
                        // 更新PPU (將會在內部渲染當前掃描線)
                        ppu.step(&mut cpu.mmu);
                    } else if ppu.ly == 144 {
                        // VBlank 開始
                        cpu.mmu
                            .write_byte(IF_REGISTER, cpu.mmu.read_byte(IF_REGISTER) | 0x01);

                        // 更新幀緩衝區
                        let ppu_buffer = ppu.get_framebuffer();
                        if ppu_buffer.len() == frame_buffer.len() {
                            frame_buffer.copy_from_slice(ppu_buffer);
                        } else {
                            println!(
                                "⚠️ 警告: PPU緩衝區大小不匹配 ({} vs {})",
                                ppu_buffer.len(),
                                frame_buffer.len()
                            );
                            // 備用方案：逐像素複製，避免越界錯誤
                            for i in 0..frame_buffer.len().min(ppu_buffer.len()) {
                                frame_buffer[i] = ppu_buffer[i];
                            }
                        }
                        frames += 1;

                        // 定期輸出診斷資訊
                        if frames % 60 == 0 {
                            println!("\n=== 幀 {} ===", frames);
                            println!(
                                "CPU 狀態: PC=0x{:04X} SP=0x{:04X}",
                                cpu.registers.pc, cpu.registers.sp
                            );

                            // VRAM 預覽
                            println!("VRAM 首個瓦片:");
                            for y in 0..2 {
                                let addr = 0x8000 + y * 16;
                                for x in 0..16 {
                                    print!("{:02X} ", cpu.mmu.read_byte(addr + x));
                                }
                                println!();
                            }
                        }
                    }

                    // PPU會在內部自行更新掃描線計數器，不需要在此更新
                }
            }
        }

        // 更新視窗顯示
        if let Err(e) = window.update_with_buffer(&frame_buffer, SCREEN_WIDTH, SCREEN_HEIGHT) {
            eprintln!("⚠️ 畫面更新失敗: {:?}", e);
            continue;
        }

        // 幀率控制
        if accumulated_time < target_frame_time {
            std::thread::sleep(target_frame_time - accumulated_time);
        }
    }

    println!("👋 模擬器正在關閉...");
    println!("✨ 模擬器已正常關閉");
    Ok(())
}

// 更新輸入狀態
fn update_input(window: &Window, mmu: &mut MMU) -> Result<(), Box<dyn Error>> {
    let mut input: u8 = 0xFF;

    // 方向鍵
    if window.is_key_down(Key::Right) {
        input &= !(1 << 0);
    }
    if window.is_key_down(Key::Left) {
        input &= !(1 << 1);
    }
    if window.is_key_down(Key::Up) {
        input &= !(1 << 2);
    }
    if window.is_key_down(Key::Down) {
        input &= !(1 << 3);
    }

    // 動作鍵
    if window.is_key_down(Key::Z) {
        input &= !(1 << 4); // A 鍵
    }
    if window.is_key_down(Key::X) {
        input &= !(1 << 5); // B 鍵
    }
    if window.is_key_down(Key::Space) {
        input &= !(1 << 6); // Select 鍵
    }
    if window.is_key_down(Key::Enter) {
        input &= !(1 << 7); // Start 鍵
    }

    // 更新輸入寄存器
    mmu.write_byte(INPUT_REGISTER, input);

    Ok(())
}
