// Game Boy CPU 模擬器 - 修復版本
use crate::mmu::MMU;

#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
}

pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
    instruction_count: u64,
}

impl CPU {
    pub fn new(mmu: MMU) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
            instruction_count: 0,
        }
    }

    pub fn step(&mut self) {
        self.execute();
        let pos = (self.registers.pc as usize) % 0x2000;
        self.mmu.vram.borrow_mut()[pos] = self.registers.pc as u8;
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom.to_vec());
    }

    pub fn execute(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
        self.instruction_count += 1;
    }

    fn fetch(&mut self) -> u8 {
        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }

    fn decode_and_execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => {} // NOP
            0x3C => {
                self.registers.a = self.registers.a.wrapping_add(1);
            } // INC A
            0x3D => {
                self.registers.a = self.registers.a.wrapping_sub(1);
            } // DEC A
            0x04 => {
                self.registers.b = self.registers.b.wrapping_add(1);
            } // INC B
            0x05 => {
                self.registers.b = self.registers.b.wrapping_sub(1);
            } // DEC B
            0x06 => {
                let n = self.fetch();
                self.registers.b = n;
            } // LD B, n
            0x0E => {
                let n = self.fetch();
                self.registers.c = n;
            } // LD C, n
            0x16 => {
                let n = self.fetch();
                self.registers.d = n;
            } // LD D, n
            0x1E => {
                let n = self.fetch();
                self.registers.e = n;
            } // LD E, n
            0x26 => {
                let n = self.fetch();
                self.registers.h = n;
            } // LD H, n
            0x2E => {
                let n = self.fetch();
                self.registers.l = n;
            } // LD L, n
            0x3E => {
                let n = self.fetch();
                self.registers.a = n;
            } // LD A, n
            0x76 => { /* HALT (暫不處理) */ }
            0xAF => {
                self.registers.a = 0;
            } // XOR A
            0xC3 => {
                // JP nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0x78 => {
                self.registers.a = self.registers.b;
            } // LD A, B
            0x79 => {
                self.registers.a = self.registers.c;
            } // LD A, C
            0x7A => {
                self.registers.a = self.registers.d;
            } // LD A, D
            0x7B => {
                self.registers.a = self.registers.e;
            } // LD A, E
            0x7C => {
                self.registers.a = self.registers.h;
            } // LD A, H
            0x7D => {
                self.registers.a = self.registers.l;
            } // LD A, L
            0x7F => { /* LD A, A (無動作) */ }
            0x18 => {
                // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
            }
            _ => println!("Opcode {:02X} not implemented", opcode),
        }
    }

    // 必需的方法
    pub fn get_enhanced_status_report(&self) -> String {
        format!(
            "CPU Status Report:\n\
             PC: 0x{:04X}, A: 0x{:02X}, B: 0x{:02X}, C: 0x{:02X}\n\
             D: 0x{:02X}, E: 0x{:02X}, H: 0x{:02X}, L: 0x{:02X}\n\
             SP: 0x{:04X}, Instructions: {}",
            self.registers.pc,
            self.registers.a,
            self.registers.b,
            self.registers.c,
            self.registers.d,
            self.registers.e,
            self.registers.h,
            self.registers.l,
            self.registers.sp,
            self.instruction_count
        )
    }

    pub fn simulate_hardware_state(&mut self) {
        let ly_addr = 0xFF44;
        let current_ly = self.mmu.read_byte(ly_addr);

        if current_ly >= 153 {
            self.mmu.write_byte(ly_addr, 0);
        } else {
            self.mmu.write_byte(ly_addr, current_ly + 1);
        }

        if current_ly == 144 {
            let if_reg = self.mmu.read_byte(0xFF0F);
            self.mmu.write_byte(0xFF0F, if_reg | 0x01);
        }
    }

    pub fn is_in_wait_loop(&self) -> bool {
        false
    }

    pub fn get_instruction_count(&self) -> u64 {
        self.instruction_count
    }

    pub fn save_performance_report(&self) {
        let report = format!(
            "Performance Report:\n\
             Total Instructions: {}\n\
             PC: 0x{:04X}\n\
             Registers: A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} H={:02X} L={:02X}\n",
            self.instruction_count,
            self.registers.pc,
            self.registers.a,
            self.registers.b,
            self.registers.c,
            self.registers.d,
            self.registers.e,
            self.registers.h,
            self.registers.l
        );

        if let Ok(mut file) = std::fs::File::create("performance_report.txt") {
            use std::io::Write;
            let _ = file.write_all(report.as_bytes());
        }
    }
}
