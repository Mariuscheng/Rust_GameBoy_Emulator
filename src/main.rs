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
use crate::joypad::GameBoyKey;
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
    let mut joypad = Joypad::new();
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
    } // 讓系統執行一段時間以啟動 ROM 初始化例程
    println!("🔄 執行 ROM 初始化例程...");
    for i in 0..2000000 {
        // 增加到 200 萬指令
        cpu.step();

        if i % 500000 == 0 {
            println!("💾 初始化進度: {} 指令", i);
            // 檢查 VRAM 狀態
            let vram_usage = cpu.mmu.vram().iter().filter(|&&b| b != 0).count();
            if vram_usage > 0 {
                println!("🧩 VRAM 已開始載入: {} 字節非零", vram_usage);

                // 如果檢測到 VRAM 有數據，可以提前結束初始化
                if vram_usage > 100 {
                    println!("✅ 檢測到足夠的 VRAM 數據，提前結束初始化");
                    break;
                }
            }

            // 檢查 CPU 狀態，防止陷入死循環
            let pc = cpu.registers.pc;
            if i > 0 && pc == 0x0214 && i % 500000 == 0 {
                println!("⚠️ 檢測到可能的死循環在 PC=0x{:04X}", pc);

                // 檢查當前執行的指令模式
                let current_opcode = cpu.mmu.read_byte(pc);
                println!("🔍 當前指令: 0x{:02X}", current_opcode);

                // 如果陷入VRAM清零循環太久，手動跳出
                if i > 1000000 {
                    println!("🚨 強制跳出可能的無限循環");
                    cpu.registers.pc = 0x0218; // 跳過循環
                }
            }
        }
    }
    println!("✅ 初始化過程完成"); // 檢查 Tetris ROM 是否正確載入了 VRAM 數據
    let vram_data = cpu.mmu.vram();
    let non_zero_count = vram_data.iter().filter(|&&b| b != 0).count();
    println!(
        "🎮 Tetris VRAM 數據檢查: {} / {} 字節非零",
        non_zero_count,
        vram_data.len()
    ); // 進行垂直線條問題分析
    println!("🔍 進行VRAM分析以診斷垂直線條問題...");
    analyze_vram_data(&vram_data);

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
    ppu.set_lcdc(initial_lcdc);

    // 設置中斷啟用寄存器 (IE) - 啟用 VBlank 和手柄中斷
    let ie_value = 0x11; // bit 0 (VBlank) + bit 4 (Joypad) = 0x01 + 0x10 = 0x11
    cpu.mmu.write_byte(0xFFFF, ie_value);

    // 啟用中斷主開關
    cpu.ime = true;

    println!(
        "🎮 中斷系統設置完成: IE=0x{:02X}, IME={}",
        ie_value, cpu.ime
    ); // 設置 BGP 為標準 Game Boy 調色板
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
        // === 按鍵處理區塊 ===
        let mut joypad_updated = false;

        // 方向鍵處理
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            if !joypad.is_key_pressed(&GameBoyKey::Up) {
                joypad.key_down(GameBoyKey::Up);
                println!("🔼 按下 上鍵");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::Up) {
            joypad.key_up(GameBoyKey::Up);
            joypad_updated = true;
        }

        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            if !joypad.is_key_pressed(&GameBoyKey::Down) {
                joypad.key_down(GameBoyKey::Down);
                println!("🔽 按下 下鍵");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::Down) {
            joypad.key_up(GameBoyKey::Down);
            joypad_updated = true;
        }

        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            if !joypad.is_key_pressed(&GameBoyKey::Left) {
                joypad.key_down(GameBoyKey::Left);
                println!("◀️ 按下 左鍵");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::Left) {
            joypad.key_up(GameBoyKey::Left);
            joypad_updated = true;
        }

        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            if !joypad.is_key_pressed(&GameBoyKey::Right) {
                joypad.key_down(GameBoyKey::Right);
                println!("▶️ 按下 右鍵");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::Right) {
            joypad.key_up(GameBoyKey::Right);
            joypad_updated = true;
        }

        // A/B 按鈕處理
        if window.is_key_down(Key::J) || window.is_key_down(Key::Z) {
            if !joypad.is_key_pressed(&GameBoyKey::A) {
                joypad.key_down(GameBoyKey::A);
                println!("🅰️ 按下 A按鈕");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::A) {
            joypad.key_up(GameBoyKey::A);
            joypad_updated = true;
        }

        if window.is_key_down(Key::K) || window.is_key_down(Key::X) {
            if !joypad.is_key_pressed(&GameBoyKey::B) {
                joypad.key_down(GameBoyKey::B);
                println!("🅱️ 按下 B按鈕");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::B) {
            joypad.key_up(GameBoyKey::B);
            joypad_updated = true;
        }

        // Select/Start 按鈕處理
        if window.is_key_down(Key::Space) {
            if !joypad.is_key_pressed(&GameBoyKey::Select) {
                joypad.key_down(GameBoyKey::Select);
                println!("📱 按下 Select");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::Select) {
            joypad.key_up(GameBoyKey::Select);
            joypad_updated = true;
        }

        if window.is_key_down(Key::Enter) {
            if !joypad.is_key_pressed(&GameBoyKey::Start) {
                joypad.key_down(GameBoyKey::Start);
                println!("▶️ 按下 Start");
                joypad_updated = true;
            }
        } else if joypad.is_key_pressed(&GameBoyKey::Start) {
            joypad.key_up(GameBoyKey::Start);
            joypad_updated = true;
        }

        // 調試按鍵（使用 is_key_pressed 而不是 is_key_down，避免重複觸發）
        static mut LAST_T_STATE: bool = false;
        let current_t_state = window.is_key_down(Key::T);
        unsafe {
            if current_t_state && !LAST_T_STATE {
                println!("\n📊 當前按鍵狀態:");
                println!("{}", joypad.generate_status_report());
            }
            LAST_T_STATE = current_t_state;
        } // 更新手柄狀態並同步到MMU
        if joypad_updated {
            joypad.update();

            // **關鍵修復**: 將手柄狀態同步到 CPU 的 MMU 中
            cpu.mmu.joypad.direction_keys = joypad.direction_keys;
            cpu.mmu.joypad.action_keys = joypad.action_keys;
            cpu.mmu.joypad.select_direction = joypad.select_direction;
            cpu.mmu.joypad.select_action = joypad.select_action;

            // **新增**: 當有按鍵按下時觸發手柄中斷
            if joypad.direction_keys != 0x0F || joypad.action_keys != 0x0F {
                let mut if_reg = cpu.mmu.read_byte(0xFF0F);
                if_reg |= 0x10; // 設置手柄中斷標誌 (bit 4)
                cpu.mmu.write_byte(0xFF0F, if_reg);
                println!("🚨 手柄按鍵觸發中斷! IF=0x{:02X}", if_reg);
            }

            // 不要強制設置手柄寄存器模式，讓 ROM 自己控制
            // ROM 會通過寫入 0xFF00 來選擇要讀取的按鍵組

            println!("🎮 手柄狀態更新完成，已同步到MMU");
        }

        // 確保 LCDC 設定正確，僅保證 LCD 顯示始終啟用
        let lcdc_value = cpu.mmu.read_byte(0xFF40);
        let fixed_lcdc = lcdc_value | 0x81; // 強制開啟 LCD 顯示和背景顯示

        if fixed_lcdc != lcdc_value {
            cpu.mmu.write_byte(0xFF40, fixed_lcdc);
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
        let mut last_pc = 0u16;
        let mut repeat_count = 0;
        let mut loop_detected = false;

        for _ in 0..1000 {
            // 檢測重複的PC
            if cpu.registers.pc == last_pc {
                repeat_count += 1;
                if repeat_count > 100 {
                    println!("⚠️ 檢測到可能的死循環在 PC=0x{:04X}", last_pc);
                    // 如果是初始化循環，強制跳出
                    if last_pc >= 0x0200 && last_pc <= 0x0300 {
                        println!("🔄 這可能是ROM初始化循環，嘗試跳過...");
                        cpu.registers.pc += 3; // 跳過當前循環
                        cpu.registers.b = 0; // B寄存器設為0，完成循環
                        loop_detected = true;
                    } // 在main.rs中改進循環檢測邏輯                    // 簡化死循環檢測 - RST 38H 問題現在由 CPU fetch 方法處理
                    if cpu.registers.pc == 0x0038 && repeat_count > 100 {
                        println!("⚠️ 檢測到長時間停留在 RST 38H (0x0038)");
                        println!("堆疊指針: SP=0x{:04X}", cpu.registers.sp);

                        // 現在直接跳到安全位置，不再嘗試手動修復堆疊
                        println!("🔧 重置到安全狀態...");
                        cpu.registers.pc = 0x0100; // 跳回ROM入口點
                        cpu.registers.sp = 0xFFFE; // 重置堆疊指針
                        repeat_count = 0;
                    }

                    // 處理其他類型的死循環
                    if cpu.registers.pc == 0x0040 && repeat_count > 100 {
                        println!("檢測到VBlank中斷處理循環");
                        println!("堆疊指針: SP=0x{:04X}", cpu.registers.sp);

                        // 強制從中斷處理返回
                        if repeat_count > 300 {
                            println!("🚨 強制從VBlank中斷返回...");
                            // 清除中斷標誌
                            let mut if_reg = cpu.mmu.read_byte(0xFF0F);
                            if_reg &= !0x01; // 清除 VBlank 中斷標誌
                            cpu.mmu.write_byte(0xFF0F, if_reg);

                            // 強制返回
                            if cpu.registers.sp < 0xFFFE - 2 {
                                let lo = cpu.mmu.read_byte(cpu.registers.sp) as u16;
                                cpu.registers.sp = cpu.registers.sp.wrapping_add(1);
                                let hi = cpu.mmu.read_byte(cpu.registers.sp) as u16;
                                cpu.registers.sp = cpu.registers.sp.wrapping_add(1);
                                cpu.registers.pc = (hi << 8) | lo;
                                cpu.ime = true; // 重新啟用中斷
                            } else {
                                // 堆疊可能已損壞，直接跳過
                                cpu.registers.pc = 0x0100; // 跳回ROM入口點
                                cpu.registers.sp = 0xFFFE; // 重置堆疊指針
                            }
                            repeat_count = 0;
                        }
                    }
                    // repeat_count 重置移到具體處理分支中
                }
            } else {
                repeat_count = 0;
                last_pc = cpu.registers.pc;
            }

            cpu.step();
            cycle_count += 4; // 模擬掃描線週期
            if cycle_count >= 456 {
                cycle_count = 0;
                let current_ly = cpu.mmu.read_byte(0xFF44);
                let next_ly = if current_ly >= 153 { 0 } else { current_ly + 1 };
                cpu.mmu.write_byte(0xFF44, next_ly);

                // VBlank 中斷只在進入 VBlank 期間觸發（LY = 144）
                if current_ly == 143 && next_ly == 144 {
                    let mut if_reg = cpu.mmu.read_byte(0xFF0F);
                    if (if_reg & 0x01) == 0 {
                        // 只有當 VBlank 中斷標誌未設置時才觸發
                        if_reg |= 0x01;
                        cpu.mmu.write_byte(0xFF0F, if_reg);
                        println!("🔄 觸發 VBlank 中斷 (LY: {} -> {})", current_ly, next_ly);
                    }
                }
            }
        }

        // 如果檢測到循環，強制運行更多指令
        if loop_detected {
            println!("🚀 強制執行更多指令以完成初始化...");
            for _ in 0..50000 {
                cpu.step();
            }
        }

        // 同步 VRAM 到 PPU
        let vram_data = cpu.mmu.vram();
        ppu.vram.copy_from_slice(&vram_data);

        // 設置 PPU 參數
        ppu.set_oam(cpu.mmu.oam());
        ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
        ppu.set_obp0(cpu.mmu.read_byte(0xFF48));
        ppu.set_obp1(cpu.mmu.read_byte(0xFF49));
        ppu.set_scx(cpu.mmu.read_byte(0xFF43));
        ppu.set_scy(cpu.mmu.read_byte(0xFF42));
        ppu.set_wx(cpu.mmu.read_byte(0xFF4B));
        ppu.set_wy(cpu.mmu.read_byte(0xFF4A));
        ppu.set_lcdc(fixed_lcdc); // 執行 PPU 渲染
        ppu.step(&mut cpu.mmu);

        // 獲取並顯示 FPS
        let fps = ppu.get_fps();
        if fps > 0 {
            let title = format!("Game Boy 模擬器 - {} FPS - {}", fps, rom_file);
            window.set_title(&title);
        }

        // 更新窗口
        window
            .update_with_buffer(ppu.get_framebuffer(), 160, 144)
            .unwrap();

        // 輸出 PPU 調試信息
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
            cpu.mmu.write_byte(0xFF47, standard_palette);
            ppu.set_bgp(standard_palette);
            println!("🎨 檢測到調色板被重置為0，已恢復為標準值 (0xE4)");
        }

        frame_count += 1;
    }

    println!("🎉 Game Boy 模擬器結束");

    // 生成最終手柄報告
    joypad.save_final_report();
    println!("✅ 手柄測試完成！調試報告已保存");

    println!("📊 總幀數: {}", frame_count);
}

// VRAM 分析函數
fn analyze_vram_data(vram_data: &[u8]) {
    println!("\n=== VRAM 垂直線條問題分析 ===");

    // 檢查背景瓦片地圖區域
    let lcdc = 0x91; // 假設LCDC值
    let bg_tile_map_base = if (lcdc & 0x08) != 0 {
        0x1C00 // $9C00-$9FFF
    } else {
        0x1800 // $9800-$9BFF
    };

    println!("背景瓦片地圖基址: 0x{:04X}", 0x8000 + bg_tile_map_base);

    // 檢查前幾個瓦片ID
    print!("前16個背景瓦片ID: ");
    for i in 0..16 {
        if bg_tile_map_base + i < vram_data.len() {
            print!("{:02X} ", vram_data[bg_tile_map_base + i]);
        }
    }
    println!();

    // 檢查瓦片數據模式
    let uses_unsigned_tiles = (lcdc & 0x10) != 0;
    println!(
        "瓦片數據模式: {}",
        if uses_unsigned_tiles {
            "無符號 (0x8000-0x8FFF)"
        } else {
            "有符號 (0x8800-0x97FF)"
        }
    );

    // 分析瓦片ID 0x00的數據
    analyze_tile_pattern_simple(vram_data, 0x00, uses_unsigned_tiles);

    // 檢查VRAM數據分布
    analyze_vram_distribution_simple(vram_data);
}

fn analyze_tile_pattern_simple(vram_data: &[u8], tile_id: u8, uses_unsigned: bool) {
    let tile_data_addr = if uses_unsigned {
        (tile_id as usize) * 16
    } else {
        let signed_id = tile_id as i8;
        0x1000 + ((signed_id as i16) + 128) as usize * 16
    };

    println!(
        "\n瓦片 ID 0x{:02X} (地址 0x{:04X}):",
        tile_id,
        0x8000 + tile_data_addr
    );

    if tile_data_addr + 15 >= vram_data.len() {
        println!("  地址超出VRAM範圍!");
        return;
    }

    // 檢查是否全零
    let mut all_zero = true;
    let mut has_vertical_pattern = true;

    for row in 0..8 {
        if tile_data_addr + row * 2 + 1 < vram_data.len() {
            let low_byte = vram_data[tile_data_addr + row * 2];
            let high_byte = vram_data[tile_data_addr + row * 2 + 1];

            if low_byte != 0 || high_byte != 0 {
                all_zero = false;
            }

            // 檢查垂直線條模式
            if low_byte != 0xAA && low_byte != 0x55 && low_byte != 0xFF && low_byte != 0x00 {
                has_vertical_pattern = false;
            }
        }
    }

    if all_zero {
        println!("  模式: 全零 (空瓦片) - 這會導致白屏或單色顯示");
    } else if has_vertical_pattern {
        println!("  模式: 垂直線條模式 (可能導致直紋)");
    } else {
        println!("  模式: 正常圖案");
    }

    // 顯示前兩行的位模式
    if tile_data_addr + 3 < vram_data.len() {
        let row0_low = vram_data[tile_data_addr];
        let row0_high = vram_data[tile_data_addr + 1];

        println!("  第0行: {:08b} {:08b}", row0_low, row0_high);

        // 解析像素顏色
        print!("  第0行像素: ");
        for bit in (0..8).rev() {
            let low_bit = (row0_low >> bit) & 1;
            let high_bit = (row0_high >> bit) & 1;
            let color_id = (high_bit << 1) | low_bit;
            print!("{}", color_id);
        }
        println!();
    }
}

fn analyze_vram_distribution_simple(vram_data: &[u8]) {
    println!("\nVRAM數據分布分析:");

    let mut zero_count = 0;
    let mut pattern_counts = [0; 256];

    for &byte in vram_data {
        if byte == 0 {
            zero_count += 1;
        }
        pattern_counts[byte as usize] += 1;
    }

    let zero_percentage = zero_count as f32 / vram_data.len() as f32 * 100.0;
    println!("  零字節: {} ({:.1}%)", zero_count, zero_percentage);

    if zero_percentage > 95.0 {
        println!("  ⚠️ 警告: VRAM中95%以上的數據為零!");
        println!("     這表明Tetris ROM的瓦片數據可能沒有正確載入到VRAM中。");
        println!("     直紋問題可能是因為瓦片數據為空，導致PPU渲染空瓦片。");
    }

    // 找出最常見的模式
    let mut sorted_patterns: Vec<(u8, usize)> = pattern_counts
        .iter()
        .enumerate()
        .map(|(i, &count)| (i as u8, count))
        .filter(|(_, count)| *count > 0)
        .collect();
    sorted_patterns.sort_by(|a, b| b.1.cmp(&a.1));

    println!("  最常見的位模式:");
    for (pattern, count) in sorted_patterns.iter().take(5) {
        if *count > 0 {
            println!("    0x{:02X} ({:08b}): {}次", pattern, pattern, count);
        }
    }
}
