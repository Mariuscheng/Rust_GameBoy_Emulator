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
    pub f: u8, // 標誌位暫存器
}

impl Registers {
    // 標誌位操作
    pub fn get_z_flag(&self) -> bool {
        (self.f & 0x80) != 0
    }
    pub fn get_n_flag(&self) -> bool {
        (self.f & 0x40) != 0
    }
    pub fn get_h_flag(&self) -> bool {
        (self.f & 0x20) != 0
    }
    pub fn get_c_flag(&self) -> bool {
        (self.f & 0x10) != 0
    }

    pub fn set_z_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x80;
        } else {
            self.f &= !0x80;
        }
    }
    pub fn set_n_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x40;
        } else {
            self.f &= !0x40;
        }
    }
    pub fn set_h_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x20;
        } else {
            self.f &= !0x20;
        }
    }
    pub fn set_c_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x10;
        } else {
            self.f &= !0x10;
        }
    }
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
            // 新增的指令實現
            0xA7 => {
                // AND A (邏輯與 A 和 A，實際上就是測試 A 的值)
                self.registers.a = self.registers.a & self.registers.a;
                // 設置標誌位: Z=結果為0, N=0, H=1, C=0
            }
            0x28 => {
                // JR Z, n (如果 Z 標誌設置則相對跳轉)
                let offset = self.fetch() as i8;
                // 暫時假設 Z=0，所以不跳轉
                // TODO: 實現標誌位系統
            }
            0xFA => {
                // LD A, (nn) (從記憶體地址 nn 載入到 A)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0xE0 => {
                // LDH (n), A (將 A 儲存到 0xFF00+n)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x85 => {
                // ADD A, L (A = A + L)
                self.registers.a = self.registers.a.wrapping_add(self.registers.l);
            }
            0x1D => {
                // DEC E (E 暫存器減一)
                self.registers.e = self.registers.e.wrapping_sub(1);
            }
            0xDA => {
                // JP C, nn (如果 C 標誌設置則跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                // 暫時假設 C=0，所以不跳轉
                // TODO: 實現標誌位系統
            }
            0xFE => {
                // CP n (比較 A 和立即數 n)
                let n = self.fetch();
                // 執行 A - n 但不保存結果，只設置標誌位
                // TODO: 實現標誌位系統
            }
            0x03 => {
                // INC BC (BC 暫存器對增一)
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_add(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            }
            0x20 => {
                // JR NZ, n (如果 Z 標誌未設置則相對跳轉)
                let offset = self.fetch() as i8;
                // 暫時假設 Z=0，所以總是跳轉
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
            }
            0x0B => {
                // DEC BC (BC 暫存器對減一)
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_sub(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            }
            0xEA => {
                // LD (nn), A (將 A 儲存到記憶體地址 nn)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xCD => {
                // CALL nn (呼叫子程式)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;

                // 將返回地址推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, self.registers.pc as u8);

                // 跳轉到目標地址
                self.registers.pc = addr;
            }
            0xF0 => {
                // LDH A, (n) (從 0xFF00+n 載入到 A)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x44 => {
                // LD B, H (將 H 暫存器的值載入 B)
                self.registers.b = self.registers.h;
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
