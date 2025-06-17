use log::info;
use std::cell::RefCell;
use std::rc::Rc;

use crate::audio::AudioProcessor;
use crate::config::Config;
use crate::cpu::{interrupts::InterruptRegisters, CPU};
use crate::error::Result; // 移除未使用的 Error
use crate::joypad::{GameBoyKey, Joypad};
use crate::mmu::MMU;
use crate::ppu::PPU;
use crate::timer::Timer;

/// Game Boy 模擬器核心
pub struct Emulator {
    pub cpu: CPU,
    pub ppu: Rc<RefCell<PPU>>,
    pub audio: AudioProcessor,
    pub timer: Timer,
    pub joypad: Joypad,
    mmu: Rc<RefCell<MMU>>,
    config: Config,
}

impl Emulator {
    /// 創建新的模擬器實例
    pub fn new(config: Config) -> Result<Self> {
        let rom_path = config.system.rom_path.clone(); // 創建核心組件
        let mmu = Rc::new(RefCell::new(MMU::default()));
        let interrupt_registers = Rc::new(RefCell::new(InterruptRegisters::new()));

        // 創建 PPU
        let video_config = config.video.clone();
        let ppu = Rc::new(RefCell::new(PPU::new(Rc::clone(&mmu), video_config)));

        // 創建其他組件
        let timer = Timer::new();
        let joypad = Joypad::new(Rc::clone(&interrupt_registers));
        let audio = AudioProcessor::new();
        // 創建 CPU
        let cpu = CPU::new(Rc::clone(&mmu), Rc::clone(&interrupt_registers));

        // 創建模擬器實例
        let emulator = Self {
            // 移除 mut
            cpu,
            ppu: Rc::clone(&ppu),
            audio,
            timer,
            joypad,
            mmu: Rc::clone(&mmu),
            config,
        };

        // 載入 ROM
        if let Some(rom_content) = std::fs::read(rom_path).ok() {
            emulator.mmu.borrow_mut().load_rom(rom_content);
        }

        Ok(emulator)
    }

    /// 執行一個模擬器步進
    pub fn step(&mut self) -> Result<()> {
        const CYCLES_PER_FRAME: u32 = 70224; // 4.194304 MHz / ~59.7275 Hz
        const MAX_CYCLES_PER_STEP: u32 = 100; // 降低每步驟的週期數以提高回應性
        const YIELD_INTERVAL: u32 = 1000; // 每執行這麼多週期就讓出 CPU

        let mut total_cycles = 0u32;
        let mut yield_counter = 0u32;
        let frame_start = std::time::Instant::now();

        while total_cycles < CYCLES_PER_FRAME {
            // 檢查是否需要讓出 CPU
            yield_counter += 1;
            if yield_counter >= YIELD_INTERVAL {
                std::thread::yield_now();
                yield_counter = 0;

                // 檢查幀執行時間是否過長
                if frame_start.elapsed() > std::time::Duration::from_millis(100) {
                    info!("幀執行時間過長，提前結束");
                    break;
                }
            }

            // 計算本次步進要執行的週期數
            let cycles_this_step =
                std::cmp::min(MAX_CYCLES_PER_STEP, CYCLES_PER_FRAME - total_cycles);

            let mut step_cycles = 0u32;
            while step_cycles < cycles_this_step {
                // 執行 CPU 指令
                let cycles = match self.cpu.step() {
                    Ok(c) => c,
                    Err(e) => {
                        let pc = self.cpu.get_pc();
                        let opcode = self.cpu.read_current_opcode()?;
                        info!(
                            "CPU 執行錯誤: {} at PC=${:04X}, opcode=${:02X}",
                            e, pc, opcode
                        );
                        self.cpu.skip_current_instruction();
                        4 // 使用一般指令的週期數
                    }
                };
                step_cycles += u32::from(cycles);

                // 更新定時器
                self.timer.update()?;

                // 更新 PPU
                let frame_done = self.ppu.borrow_mut().tick()?;
                if frame_done {
                    return Ok(());
                }

                // 更新音效處理器
                if self.config.audio.enable_sound {
                    self.audio.step(4); // 移除 ? 運算符，因為 step 方法不回傳 Result
                }
            }

            total_cycles += step_cycles;
        }

        Ok(())
    }
    /// 重置模擬器
    pub fn reset(&mut self) -> Result<()> {
        self.cpu.reset();
        self.ppu.borrow_mut().reset();
        self.audio.reset();
        self.timer.reset();
        self.joypad.reset();
        Ok(())
    }

    /// 暫停模擬器
    pub fn pause(&mut self) {
        // TODO: 實現暫停功能
    }

    /// 保存遊戲狀態
    pub fn save_state(&self, _path: &str) -> Result<()> {
        // TODO: 實現存檔功能
        Ok(())
    }

    /// 載入遊戲狀態
    pub fn load_state(&mut self, _path: &str) -> Result<()> {
        // TODO: 實現讀檔功能
        Ok(())
    }
    /// 獲取當前幀緩衝區（RGBA8888格式）
    pub fn get_frame(&self) -> Result<Vec<u8>> {
        if let Ok(ppu) = self.ppu.try_borrow() {
            ppu.get_display_buffer()
                .map(|buffer| buffer.to_vec())
                .map_err(Into::into)
        } else {
            Ok(vec![0; 160 * 144 * 4]) // 返回黑色幀
        }
    }

    // 按鍵處理函數
    pub fn set_button_a(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::A, pressed);
    }

    pub fn set_button_b(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::B, pressed);
    }

    pub fn set_button_select(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::Select, pressed);
    }

    pub fn set_button_start(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::Start, pressed);
    }

    pub fn set_button_up(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::Up, pressed);
    }

    pub fn set_button_down(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::Down, pressed);
    }

    pub fn set_button_left(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::Left, pressed);
    }

    pub fn set_button_right(&mut self, pressed: bool) {
        self.joypad.set_button_state(GameBoyKey::Right, pressed);
    }
}
