use minifb::{Window, WindowOptions};
use std::fs;

// 模擬器核心結構
mod cpu {
    use super::mmu::MMU;

    // CPU 寄存器
    #[allow(dead_code)] // 忽略未使用欄位警告
    #[derive(Default)]
    pub struct Registers {
        pub a: u8,  // 累加器，公開
        pub f: u8,  // 旗標，公開
        pub pc: u16, // 程式計數器，公開
    }

    pub struct CPU {
        registers: Registers,
        mmu: MMU,
    }

    impl CPU {
        pub fn new(mmu: MMU) -> Self {
            CPU {
                registers: Registers::default(),
                mmu,
            }
        }

        // 公開方法：載入 ROM
        pub fn load_rom(&mut self, rom: &[u8]) {
            self.mmu.load_rom(rom);
        }

        // 公開 getter：獲取寄存器
        pub fn registers(&self) -> &Registers {
            &self.registers
        }

        // 執行單條指令
        pub fn step(&mut self) -> u8 {
            let opcode = self.mmu.read_byte(self.registers.pc);
            self.registers.pc = self.registers.pc.wrapping_add(1);
            match opcode {
                0x00 => {
                    // NOP: 無操作
                    4
                }
                0x3C => {
                    // INC A: 遞增累加器
                    self.registers.a = self.registers.a.wrapping_add(1);
                    4
                }
                0x3E => {
                    // LD A, n: 載入立即數到 A
                    let value = self.mmu.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.a = value;
                    8
                }
                _ => {
                    println!("未知指令: 0x{:02X}", opcode);
                    4
                }
            }
        }
    }
}

mod mmu {
    pub struct MMU {
        memory: [u8; 0x10000], // 64KB 記憶體
    }

    #[allow(dead_code)] // 忽略未使用方法警告
    impl MMU {
        pub fn new() -> Self {
            MMU {
                memory: [0; 0x10000],
            }
        }

        pub fn read_byte(&self, addr: u16) -> u8 {
            self.memory[addr as usize]
        }

        pub fn write_byte(&mut self, addr: u16, value: u8) {
            self.memory[addr as usize] = value;
        }

        pub fn load_rom(&mut self, rom: &[u8]) {
            // 將 ROM 數據複製到記憶體（0x0000 開始）
            for (i, &byte) in rom.iter().enumerate().take(0x8000) { // 限制為 32KB
                self.memory[i] = byte;
            }

            // 解析 ROM Header（簡單範例）
            let title = &self.memory[0x134..0x143]
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c as char)
                .collect::<String>();
            let cartridge_type = self.memory[0x147];
            println!("ROM 標題: {}", title);
            println!("卡匣類型: 0x{:02X}", cartridge_type);
        }
    }
}

fn main() {
    // 初始化 MMU 和 CPU
    let mmu = mmu::MMU::new();
    let mut cpu = cpu::CPU::new(mmu);

    // 載入 .gb 檔案
    let rom = fs::read("rom.gb").unwrap_or_else(|e| {
        println!("無法讀取 rom.gb: {}", e);
        vec![0x00, 0x3C, 0x00] // 預設簡單 ROM
    });
    cpu.load_rom(&rom);

    // 主迴圈（執行 5 次）
    for _ in 0..5 {
        let cycles = cpu.step();
        let registers = cpu.registers();
        println!("PC: 0x{:04X}, A: 0x{:02X}, Cycles: {}", registers.pc, registers.a, cycles);
    }

    // 初始化 minifb 視窗（模擬 PPU）
    let mut window = Window::new("Game Boy Emulator", 160, 144, WindowOptions::default()).unwrap();
    let buffer: Vec<u32> = vec![0xFFFFFF; 160 * 144]; // 全白畫面

    // 簡單渲染
    while window.is_open() {
        window.update_with_buffer(&buffer, 160, 144).unwrap();
    }
}