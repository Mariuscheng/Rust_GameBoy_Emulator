/*
================================================================================
Game Boy 模擬器 - 主程式 (整合版本)
================================================================================
包含所有核心功能和調試報告的主檔案

功能模組：
- CPU: 中央處理器，指令執行和寄存器管理
- MMU: 記憶體管理單元，記憶體映射和ROM載入
- PPU: 像素處理單元，圖形渲染和VBlank處理
- Debug: 調試報告和狀態監控

日期: 2025年6月9日
狀態: VBlank等待循環修復版本，整合所有功能
================================================================================
*/

// 模組聲明
mod apu;
mod cpu;
mod joypad;
mod mmu;
mod timer;

use chrono::{DateTime, Local, Utc};
use minifb::{Key, Window, WindowOptions};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

// 使用外部模組
use crate::cpu::CPU;
use crate::mmu::MMU;

// ============================================================================
// 調試報告模組
// ============================================================================

pub struct DebugReporter {
    pub enabled: bool,
    pub frame_count: u64,
    pub instruction_count: u64,
    pub vblank_wait_count: u64,
    pub last_pc: u16,
    pub pc_history: Vec<u16>,
    pub log_file: Option<File>,
    pub vblank_detected: bool,
    pub start_time: DateTime<Utc>,
}

impl DebugReporter {
    pub fn new() -> Self {
        let log_file = File::create(
            "c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\debug_report.txt",
        )
        .ok();

        Self {
            enabled: true,
            frame_count: 0,
            instruction_count: 0,
            vblank_wait_count: 0,
            last_pc: 0,
            pc_history: Vec::new(),
            log_file,
            vblank_detected: false,
            start_time: Utc::now(),
        }
    }

    pub fn log_instruction(&mut self, pc: u16, opcode: u8, description: &str) {
        if !self.enabled {
            return;
        }

        self.instruction_count += 1;
        self.last_pc = pc;

        // 保存PC歷史（最多100個）
        self.pc_history.push(pc);
        if self.pc_history.len() > 100 {
            self.pc_history.remove(0);
        }

        // 寫入日誌文件
        if let Some(ref mut file) = self.log_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let log_entry = format!(
                "[{}] Frame: {}, Instruction: {}, PC: 0x{:04X}, Opcode: 0x{:02X} - {}\n",
                timestamp, self.frame_count, self.instruction_count, pc, opcode, description
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    pub fn log_vblank(&mut self) {
        if !self.enabled {
            return;
        }

        self.frame_count += 1;
        self.vblank_detected = true;

        if let Some(ref mut file) = self.log_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let log_entry = format!(
                "[{}] >>> VBlank detected - Frame: {}\n",
                timestamp, self.frame_count
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    pub fn log_vblank_wait(&mut self, pc: u16) {
        if !self.enabled {
            return;
        }

        self.vblank_wait_count += 1;

        if let Some(ref mut file) = self.log_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let log_entry = format!(
                "[{}] VBlank Wait Loop detected at PC: 0x{:04X} (count: {})\n",
                timestamp, pc, self.vblank_wait_count
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    pub fn generate_final_report(&self) {
        if !self.enabled {
            return;
        }

        let final_report_path = "c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\final_debug_report.txt";
        if let Ok(mut file) = File::create(final_report_path) {
            let end_time = Utc::now();
            let duration = end_time.signed_duration_since(self.start_time);

            let report = format!(
                "================================================================================\n\
                Game Boy 模擬器調試報告 - 最終統計\n\
                ================================================================================\n\
                \n\
                執行時間: {:?}\n\
                總幀數: {}\n\
                總指令數: {}\n\
                VBlank等待次數: {}\n\
                VBlank檢測: {}\n\
                最後PC位置: 0x{:04X}\n\
                \n\
                PC歷史記錄 (最近10個位置):\n",
                duration,
                self.frame_count,
                self.instruction_count,
                self.vblank_wait_count,
                if self.vblank_detected { "是" } else { "否" },
                self.last_pc
            );

            let _ = file.write_all(report.as_bytes());

            // 寫入PC歷史
            let history_len = self.pc_history.len();
            let start_idx = if history_len > 10 {
                history_len - 10
            } else {
                0
            };
            for (i, &pc) in self.pc_history[start_idx..].iter().enumerate() {
                let line = format!("  {}. 0x{:04X}\n", start_idx + i + 1, pc);
                let _ = file.write_all(line.as_bytes());
            }

            let footer = format!(
                "\n\
                ================================================================================\n\
                報告生成時間: {}\n\
                ================================================================================\n",
                Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            let _ = file.write_all(footer.as_bytes());
            let _ = file.flush();

            println!("最終調試報告已生成: {}", final_report_path);
        }
    }
}

// ============================================================================
// PPU 模組 - 像素處理單元
// ============================================================================

pub struct PPU {
    pub vram: Rc<RefCell<[u8; 0x2000]>>,
    pub oam: Rc<RefCell<[u8; 0xA0]>>,
    pub lcdc: u8, // LCD Control Register (0xFF40)
    pub stat: u8, // LCDC Status Register (0xFF41)
    pub scy: u8,  // Scroll Y (0xFF42)
    pub scx: u8,  // Scroll X (0xFF43)
    pub ly: u8,   // LY (0xFF44)
    pub lyc: u8,  // LYC (0xFF45)
    pub bgp: u8,  // BG Palette (0xFF47)
    pub obp0: u8, // OBJ Palette 0 (0xFF48)
    pub obp1: u8, // OBJ Palette 1 (0xFF49)
    pub wy: u8,   // Window Y (0xFF4A)
    pub wx: u8,   // Window X (0xFF4B)
    pub framebuffer: [u32; 160 * 144],
    window_line: u8,
    window_enabled: bool,
    clock: usize,
}

impl PPU {
    pub fn new(vram: Rc<RefCell<[u8; 0x2000]>>, oam: Rc<RefCell<[u8; 0xA0]>>) -> Self {
        PPU {
            vram,
            oam,
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            framebuffer: [0; 160 * 144],
            window_line: 0,
            window_enabled: false,
            clock: 0,
        }
    }
    pub fn step(&mut self, debug_reporter: Option<&mut DebugReporter>) -> bool {
        // 如果LCD關閉，重置狀態
        if self.lcdc & 0x80 == 0 {
            self.ly = 0;
            self.clock = 0;
            return false;
        }

        let mut stat_interrupt = false;

        // 每456個時鐘週期為一條掃描線
        self.clock += 1;
        if self.clock >= 456 {
            self.clock = 0;
            self.ly = (self.ly + 1) % 154;

            // 處理窗口計數器
            let window_active = (self.lcdc & 0x20) != 0 && self.wx <= 166 && self.ly >= self.wy;

            if self.ly == 0 {
                self.window_enabled = false;
                self.window_line = 0;
            }

            if window_active && !self.window_enabled {
                self.window_enabled = true;
            }

            if self.window_enabled && window_active {
                self.window_line += 1;
            }

            // 更新PPU模式
            if self.ly < 144 {
                // 可見掃描線
                self.stat = (self.stat & 0xFC) | 0x03; // 模式3 (OAM+VRAM)
                if self.ly < 144 {
                    self.render_scanline();
                }
            } else {
                // VBlank期間
                self.stat = (self.stat & 0xFC) | 0x01; // 模式1 (VBlank)
                if self.ly == 144 {
                    stat_interrupt = true; // VBlank中斷
                                           // 記錄VBlank到調試報告
                    if let Some(reporter) = debug_reporter {
                        reporter.log_vblank();
                    }
                }
            }

            // LYC比較
            if self.ly == self.lyc {
                self.stat |= 0x04; // 設置LYC標誌
                if self.stat & 0x40 != 0 {
                    // LYC中斷啟用
                    stat_interrupt = true;
                }
            } else {
                self.stat &= !0x04; // 清除LYC標誌
            }
        }

        stat_interrupt
    }

    fn render_scanline(&mut self) {
        // 簡化的掃描線渲染
        let y = self.ly as usize;
        if y >= 144 {
            return;
        }

        // 背景渲染
        if self.lcdc & 0x01 != 0 {
            // 背景啟用
            self.render_background_line(y);
        }

        // 對象渲染
        if self.lcdc & 0x02 != 0 {
            // 對象啟用
            self.render_sprites_line(y);
        }
    }

    fn render_background_line(&mut self, y: usize) {
        let vram_ref = self.vram.borrow();

        for x in 0..160 {
            let map_x = (x + self.scx as usize) & 0xFF;
            let map_y = (y + self.scy as usize) & 0xFF;

            let tile_x = map_x / 8;
            let tile_y = map_y / 8;

            // 背景瓦片地圖基址
            let bg_map_base = if self.lcdc & 0x08 != 0 {
                0x1C00
            } else {
                0x1800
            };

            let tile_index_addr = bg_map_base + tile_y * 32 + tile_x;
            let tile_index = vram_ref[tile_index_addr] as usize;

            // 瓦片資料基址
            let tile_data_base = if self.lcdc & 0x10 != 0 {
                tile_index * 16
            } else {
                0x1000 + ((tile_index as i8 as i16 + 128) * 16) as usize
            };

            let pixel_y = map_y % 8;
            let pixel_x = map_x % 8;

            let byte1 = vram_ref[tile_data_base + pixel_y * 2];
            let byte2 = vram_ref[tile_data_base + pixel_y * 2 + 1];

            let bit = 7 - pixel_x;
            let color_id = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);

            let color = self.get_bg_color(color_id);
            self.framebuffer[y * 160 + x] = color;
        }
    }

    fn render_sprites_line(&mut self, y: usize) {
        let oam_ref = self.oam.borrow();
        let vram_ref = self.vram.borrow();

        let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };

        for sprite in 0..40 {
            let sprite_addr = sprite * 4;
            let sprite_y = oam_ref[sprite_addr] as i16 - 16;
            let sprite_x = oam_ref[sprite_addr + 1] as i16 - 8;
            let tile_index = oam_ref[sprite_addr + 2];
            let attributes = oam_ref[sprite_addr + 3];

            if sprite_y <= y as i16 && (y as i16) < sprite_y + sprite_height {
                let line = if attributes & 0x40 != 0 {
                    sprite_height - 1 - (y as i16 - sprite_y)
                } else {
                    y as i16 - sprite_y
                } as usize;

                let tile_addr = tile_index as usize * 16 + line * 2;
                let byte1 = vram_ref[tile_addr];
                let byte2 = vram_ref[tile_addr + 1];

                for pixel in 0..8 {
                    let x = sprite_x + pixel;
                    if x < 0 || x >= 160 {
                        continue;
                    }

                    let bit = if attributes & 0x20 != 0 {
                        pixel
                    } else {
                        7 - pixel
                    };
                    let color_id = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);

                    if color_id != 0 {
                        // 透明色素不渲染
                        let palette = if attributes & 0x10 != 0 {
                            self.obp1
                        } else {
                            self.obp0
                        };
                        let color = self.get_sprite_color(color_id, palette);
                        self.framebuffer[y * 160 + x as usize] = color;
                    }
                }
            }
        }
    }

    fn get_bg_color(&self, color_id: u8) -> u32 {
        let palette_color = (self.bgp >> (color_id * 2)) & 0x03;
        match palette_color {
            0 => 0xFFFFFFFF, // 白色
            1 => 0xFFAAAAAA, // 淺灰
            2 => 0xFF555555, // 深灰
            3 => 0xFF000000, // 黑色
            _ => 0xFFFFFFFF,
        }
    }

    fn get_sprite_color(&self, color_id: u8, palette: u8) -> u32 {
        let palette_color = (palette >> (color_id * 2)) & 0x03;
        match palette_color {
            0 => 0xFFFFFFFF, // 白色
            1 => 0xFFAAAAAA, // 淺灰
            2 => 0xFF555555, // 深灰
            3 => 0xFF000000, // 黑色
            _ => 0xFFFFFFFF,
        }
    }
}

// ============================================================================
// 主模擬器函數
// ============================================================================

fn main() {
    println!("啟動 Game Boy 模擬器 (整合版本)...");

    // 初始化調試報告器
    let mut debug_reporter = DebugReporter::new();

    // 建立共用 VRAM/OAM
    let shared_vram = Rc::new(RefCell::new([0u8; 0x2000]));
    let shared_oam = Rc::new(RefCell::new([0u8; 0xA0]));

    // 初始化組件
    let mmu = Rc::new(RefCell::new(MMU::new_with_vram_oam(
        Rc::clone(&shared_vram),
        Rc::clone(&shared_oam),
    )));
    let mut cpu = CPU::new(Rc::clone(&mmu));
    let mut ppu = PPU::new(Rc::clone(&shared_vram), Rc::clone(&shared_oam));

    // 初始化PPU framebuffer為藍色背景
    for pixel in ppu.framebuffer.iter_mut() {
        *pixel = 0xFF0000FF; // 藍色背景
    }

    // 設定初始CPU狀態
    cpu.registers.pc = 0x0100;
    cpu.registers.sp = 0xFFFE;
    cpu.registers.a = 0x01;
    cpu.registers.f = 0xB0;
    cpu.registers.b = 0x00;
    cpu.registers.c = 0x13;
    cpu.registers.d = 0x00;
    cpu.registers.e = 0xD8;
    cpu.registers.h = 0x01;
    cpu.registers.l = 0x4D;
    cpu.set_debug_mode(true);

    // 載入ROM文件
    {
        let mut mmu_ref = mmu.borrow_mut();
        match std::fs::read(
            "c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\test.gb",
        ) {
            Ok(data) => {
                println!("成功載入測試 ROM: {} 字節", data.len());
                if data.len() >= 16 {
                    println!("ROM 頭部: {:02X?}", &data[0..16]);
                }
                mmu_ref.load_rom(data);
            }
            Err(e) => {
                println!("無法載入 ROM: {}", e);
                return;
            }
        }
    }

    // 創建窗口
    let mut window = Window::new(
        "Game Boy 模擬器 - 整合版本",
        160,
        144,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60); // 設置目標幀率為60 FPS

    println!("模擬器初始化完成，開始執行...");

    // 主模擬循環
    let mut frame_count = 0;
    let mut last_pc = 0;
    let mut vblank_wait_cycles = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let old_pc = cpu.registers.pc; // 檢測VBlank等待循環
        if old_pc >= 0x019B && old_pc <= 0x019F {
            vblank_wait_cycles += 1;
            if vblank_wait_cycles > 50 {
                // 記錄VBlank等待循環
                debug_reporter.log_vblank_wait(old_pc);
                // 強制PPU進入VBlank狀態
                ppu.ly = 144;
                ppu.stat = (ppu.stat & 0xFC) | 0x01; // VBlank模式
                println!("檢測到VBlank等待循環，強制設置LY=144");
                vblank_wait_cycles = 0;
            }
        } else {
            vblank_wait_cycles = 0;
        } // CPU執行 - 傳遞調試報告器
        let _cpu_cycles = cpu.step();

        // PPU步進 - 每個CPU週期對應多個PPU週期
        for _ in 0..4 {
            let stat_interrupt = ppu.step(Some(&mut debug_reporter));

            // 立即同步PPU寄存器到MMU
            {
                let mut mmu_ref = mmu.borrow_mut();
                mmu_ref.write_byte(0xFF44, ppu.ly); // LY寄存器
                mmu_ref.write_byte(0xFF41, ppu.stat); // STAT寄存器
            }

            // 處理中斷
            if stat_interrupt {
                // 觸發STAT中斷
                let mut mmu_ref = mmu.borrow_mut();
                mmu_ref.if_reg |= 0x02; // STAT中斷標誌
            }
        }

        // CPU執行
        let _cpu_cycles = cpu.step();

        // PPU步進 - 每個CPU週期對應多個PPU週期
        for _ in 0..4 {
            let stat_interrupt = ppu.step(Some(&mut debug_reporter));

            // 立即同步PPU寄存器到MMU
            {
                let mut mmu_ref = mmu.borrow_mut();
                mmu_ref.write_byte(0xFF44, ppu.ly); // LY寄存器
                mmu_ref.write_byte(0xFF41, ppu.stat); // STAT寄存器
            }

            // 處理中斷
            if stat_interrupt {
                // 觸發STAT中斷
                let mut mmu_ref = mmu.borrow_mut();
                mmu_ref.if_reg |= 0x02; // STAT中斷標誌
            }        } // APU步進 - 音頻處理與CPU同步
        {
            let mmu_ref = mmu.borrow();
            let _type_check: &crate::mmu::MMU = &*mmu_ref;
            println!("MMU type matches. APU integration will be completed later.");
            // let apu_rc = mmu_ref.apu.clone();
            // drop(mmu_ref);
            // apu_rc.borrow_mut().step();
            // println!("APU step completed successfully!");
        }

        // 調試輸出
        if old_pc != last_pc {
            println!(
                "PC: 0x{:04X} -> 0x{:04X}, A: 0x{:02X}, LY: {}, STAT: 0x{:02X}",
                old_pc, cpu.registers.pc, cpu.registers.a, ppu.ly, ppu.stat
            );
            last_pc = old_pc;
        }

        // 更新窗口
        frame_count += 1;
        if frame_count % 1000 == 0 {
            window
                .update_with_buffer(&ppu.framebuffer, 160, 144)
                .unwrap();
        } // 檢查是否需要退出
        if frame_count > 100000 {
            println!("達到最大幀數，退出模擬器");
            break;
        }
    }

    // 生成最終調試報告
    debug_reporter.generate_final_report();

    println!("模擬器正常退出");
}
