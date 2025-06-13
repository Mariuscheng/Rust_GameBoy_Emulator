// Game Boy CPU 模擬器 - 根據 CPU 文檔改進版本
use crate::mmu::MMU;

const VBLANK_VECTOR: u16 = 0x0040;
const LCD_VECTOR: u16 = 0x0048;
const TIMER_VECTOR: u16 = 0x0050;
const SERIAL_VECTOR: u16 = 0x0058;
const JOYPAD_VECTOR: u16 = 0x0060;

// 中斷標誌位
const VBLANK_FLAG: u8 = 0x01;
const LCD_FLAG: u8 = 0x02;
const TIMER_FLAG: u8 = 0x04;
const SERIAL_FLAG: u8 = 0x08;
const JOYPAD_FLAG: u8 = 0x10;

#[derive(Default, Debug, Clone)]
pub struct Registers {
    pub a: u8,   // 累加器
    pub b: u8,   // B 寄存器
    pub c: u8,   // C 寄存器
    pub d: u8,   // D 寄存器
    pub e: u8,   // E 寄存器
    pub h: u8,   // H 寄存器
    pub l: u8,   // L 寄存器
    pub f: u8,   // 標誌位寄存器
    pub sp: u16, // 堆疊指針
    pub pc: u16, // 程序計數器
}

// CPU 狀態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CPUState {
    Running,
    Halted,
    Stopped,
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

    // 16-bit 暫存器對操作
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xF0) as u8; // 只保留高4位
    }

    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
}

// CPU 執行追蹤
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    pub pc: u16,
    pub opcode: u8,
    pub registers: Registers,
    pub cycles: u32,
}

pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
    pub state: CPUState,
    pub ime: bool,
    pub instruction_count: u64,
    pub total_cycles: u64,
    pub ei_delay: bool,
    pub halt_bug: bool,
    // 追蹤系統
    pub trace_enabled: bool,
    pub execution_trace: Vec<ExecutionTrace>,
    pub vram_writes: Vec<(u16, u8, u16)>, // (addr, value, pc)
}

impl CPU {
    pub fn new(mmu: MMU) -> Self {
        let mut cpu = Self {
            registers: Registers::default(),
            mmu,
            state: CPUState::Running,
            ime: false,
            instruction_count: 0,
            total_cycles: 0,
            ei_delay: false,
            halt_bug: false,
            trace_enabled: true, // 開啟追蹤以便調試
            execution_trace: Vec::with_capacity(1000),
            vram_writes: Vec::with_capacity(100),
        };

        // 設置 Game Boy 的初始寄存器值
        cpu.registers.a = 0x01;
        cpu.registers.f = 0xB0;
        cpu.registers.set_bc(0x0013);
        cpu.registers.set_de(0x00D8);
        cpu.registers.set_hl(0x014D);
        cpu.registers.sp = 0xFFFE;
        cpu.registers.pc = 0x0100; // 從 ROM 入口點開始

        println!("CPU 初始化完成:");
        println!(
            "  A: 0x{:02X}  F: 0x{:02X}",
            cpu.registers.a, cpu.registers.f
        );
        println!(
            "  B: 0x{:02X}  C: 0x{:02X}",
            cpu.registers.b, cpu.registers.c
        );
        println!(
            "  D: 0x{:02X}  E: 0x{:02X}",
            cpu.registers.d, cpu.registers.e
        );
        println!(
            "  H: 0x{:02X}  L: 0x{:02X}",
            cpu.registers.h, cpu.registers.l
        );
        println!("  SP: 0x{:04X}", cpu.registers.sp);
        println!("  PC: 0x{:04X}", cpu.registers.pc);

        cpu
    }

    /// 載入 ROM 到 CPU
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.mmu.load_rom(rom_data.to_vec());
    }

    // 開啟追蹤
    pub fn enable_tracing(&mut self) {
        self.trace_enabled = true;
    }

    // 關閉追蹤
    pub fn disable_tracing(&mut self) {
        self.trace_enabled = false;
    }

    // 記錄執行追蹤
    fn record_trace(&mut self, opcode: u8, cycles: u32) {
        if !self.trace_enabled {
            return;
        }

        if self.execution_trace.len() >= 1000 {
            self.execution_trace.remove(0);
        }

        self.execution_trace.push(ExecutionTrace {
            pc: self.registers.pc,
            opcode,
            registers: self.registers.clone(),
            cycles,
        });
    }
    pub fn step(&mut self) -> u8 {
        // 處理 EI 指令的延遲
        if self.ei_delay {
            self.ime = true;
            self.ei_delay = false;
        }

        // 獲取當前操作碼並執行詳細調試
        let opcode = self.mmu.read_byte(self.registers.pc);

        // 在關鍵範圍內輸出詳細調試信息
        if self.registers.pc <= 0x100 || self.instruction_count < 1000 {
            self.debug_instruction(opcode);
        }

        // 檢查中斷
        if self.ime && self.state != CPUState::Stopped {
            if let Some(interrupt_cycles) = self.handle_interrupts() {
                self.total_cycles += interrupt_cycles as u64;
                return interrupt_cycles;
            }
        }

        match self.state {
            CPUState::Running => {
                let cycles = self.execute();
                self.total_cycles += cycles as u64;
                cycles
            }
            CPUState::Halted => {
                // HALT 模式下仍然檢查中斷
                if self.ime {
                    if let Some(interrupt_cycles) = self.handle_interrupts() {
                        self.state = CPUState::Running;
                        self.total_cycles += interrupt_cycles as u64;
                        return interrupt_cycles;
                    }
                }
                4 // HALT 模式消耗 1 機器週期
            }
            CPUState::Stopped => {
                // STOP 模式,只能通過按鈕中斷喚醒
                4
            }
        }
    }

    pub fn execute(&mut self) -> u8 {
        let opcode = self.fetch();
        let cycles = self.decode_and_execute(opcode);
        self.instruction_count += 1;
        cycles
    }

    fn fetch(&mut self) -> u8 {
        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }

    // 中斷處理
    fn handle_interrupts(&mut self) -> Option<u8> {
        let ie = self.mmu.read_byte(0xFFFF); // 中斷啟用寄存器
        let if_reg = self.mmu.read_byte(0xFF0F); // 中斷標誌寄存器

        let interrupts = ie & if_reg;
        if interrupts == 0 {
            return None;
        }

        // 如果 CPU 處於 HALT 狀態,中斷會喚醒它
        if self.state == CPUState::Halted {
            self.state = CPUState::Running;
        }

        // 如果中斷被啟用,處理優先級最高的中斷
        if self.ime {
            self.ime = false; // 禁用中斷

            // 推入當前 PC 到堆疊
            self.push_stack(self.registers.pc);

            // 中斷處理,按優先級順序
            let interrupt_handlers = [
                (VBLANK_FLAG, VBLANK_VECTOR),
                (LCD_FLAG, LCD_VECTOR),
                (TIMER_FLAG, TIMER_VECTOR),
                (SERIAL_FLAG, SERIAL_VECTOR),
                (JOYPAD_FLAG, JOYPAD_VECTOR),
            ];

            for &(flag, vector) in &interrupt_handlers {
                if interrupts & flag != 0 {
                    self.mmu.write_byte(0xFF0F, if_reg & !flag);
                    self.registers.pc = vector;
                    return Some(20); // 5 機器週期
                }
            }
        }

        None
    }

    // 堆疊操作輔助方法
    fn push_stack(&mut self, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.mmu.write_byte(self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.mmu.write_byte(self.registers.sp, value as u8);
    }

    fn pop_stack(&mut self) -> u16 {
        let lo = self.mmu.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        let hi = self.mmu.read_byte(self.registers.sp) as u16;
        self.registers.sp = self.registers.sp.wrapping_add(1);
        (hi << 8) | lo
    }
    fn decode_and_execute(&mut self, opcode: u8) -> u8 {
        // 變量使用以避免重複計算
        let cycles = match opcode {
            0x5E => {
                // LD E,(HL)
                let addr = self.registers.get_hl();
                self.registers.e = self.mmu.read_byte(addr);
                8
            }
            0x98 => {
                // SBC A,B
                let value = self.registers.b;
                let carry = if self.registers.get_c_flag() { 1 } else { 0 };
                let result = self.registers.a.wrapping_sub(value).wrapping_sub(carry);
                let half_carry = (self.registers.a & 0x0F) < ((value & 0x0F) + carry);
                let full_carry = (self.registers.a as u16) < (value as u16 + carry as u16);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag(half_carry);
                self.registers.set_c_flag(full_carry);

                self.registers.a = result;
                4
            }
            0x5F => {
                // LD E,A
                self.registers.e = self.registers.a;
                4
            }
            0x16 => {
                // LD D,n
                let n = self.fetch();
                self.registers.d = n;
                8
            }
            0x56 => {
                // LD D,(HL)
                let addr = self.registers.get_hl();
                self.registers.d = self.mmu.read_byte(addr);
                8
            }
            0x62 => {
                // LD H,D
                self.registers.h = self.registers.d;
                4
            }
            0x60 => {
                // LD H,B
                self.registers.h = self.registers.b;
                4
            }
            0x02 => {
                // LD (BC),A
                let addr = self.registers.get_bc();
                self.mmu.write_byte(addr, self.registers.a);
                8
            }
            0x24 => {
                // INC H
                let result = self.registers.h.wrapping_add(1);
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.h & 0x0F) == 0x0F);
                self.registers.h = result;
                4
            }
            0x34 => {
                // INC (HL)
                let addr = self.registers.get_hl();
                let value = self.mmu.read_byte(addr);
                let result = value.wrapping_add(1);
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((value & 0x0F) == 0x0F);
                self.mmu.write_byte(addr, result);
                12
            }
            0x33 => {
                // INC SP
                self.registers.sp = self.registers.sp.wrapping_add(1);
                8
            }
            0x0A => {
                // LD A,(BC)
                let addr = self.registers.get_bc();
                self.registers.a = self.mmu.read_byte(addr);
                8
            }
            0x19 => {
                // ADD HL,DE
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                let result = hl.wrapping_add(de);

                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag((hl & 0x0FFF) + (de & 0x0FFF) > 0x0FFF);
                self.registers.set_c_flag(hl as u32 + de as u32 > 0xFFFF);

                self.registers.set_hl(result);
                8
            }
            0x65 => {
                // LD H,L
                self.registers.h = self.registers.l;
                4
            }
            0x2E => {
                // LD L,n
                let n = self.fetch();
                self.registers.l = n;
                8
            }
            0xE1 => {
                // POP HL
                let value = self.pop_stack();
                self.registers.set_hl(value);
                12
            }
            0xC0 => {
                // RET NZ
                if !self.registers.get_z_flag() {
                    self.registers.pc = self.pop_stack();
                    20
                } else {
                    8
                }
            }
            // ...rest of the existing code...
            _ => {
                println!(
                    "警告：未實現的指令 0x{:02X} 在 PC=0x{:04X}",
                    opcode,
                    self.registers.pc.wrapping_sub(1)
                );
                4
            }
        };

        // 新增: 記錄指令執行
        self.record_trace(opcode, 0);

        match opcode {
            // === 8-bit 指令 ===
            0x0F => {
                // RRCA
                let carry = self.registers.a & 0x01; // 取得最低位
                self.registers.a = (self.registers.a >> 1) | (carry << 7); // 右移並將原最低位放到最高位

                self.registers.set_z_flag(false); // RRCA 不設置零標誌
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry == 1);

                4
            }
            0x07 => {
                // RLCA - 累加器向左旋轉
                let carry = (self.registers.a & 0x80) >> 7; // 取得最高位
                self.registers.a = (self.registers.a << 1) | carry; // 左移並將原最高位放到最低位

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry == 1);

                4
            }
            0x08 => {
                // LD (nn), SP - 將 SP 值存入指定記憶體位置
                let low = self.fetch();
                let high = self.fetch();
                let addr = ((high as u16) << 8) | (low as u16);

                // 將 SP 的低 8 位存入 addr
                self.mmu.write_byte(addr, (self.registers.sp & 0xFF) as u8);
                // 將 SP 的高 8 位存入 addr+1
                self.mmu
                    .write_byte(addr + 1, (self.registers.sp >> 8) as u8);

                20 // 需要 5 個機器週期
            }
            0x17 => {
                // RLA - 透過進位旗標向左旋轉累加器
                let old_carry = if self.registers.get_c_flag() { 1 } else { 0 };
                let new_carry = (self.registers.a & 0x80) >> 7;
                self.registers.a = (self.registers.a << 1) | old_carry;

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(new_carry == 1);

                4
            }
            0x1F => {
                // RRA - 透過進位旗標向右旋轉累加器
                let old_carry = if self.registers.get_c_flag() { 1 } else { 0 };
                let new_carry = self.registers.a & 0x01;
                self.registers.a = (self.registers.a >> 1) | (old_carry << 7);

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(new_carry == 1);

                4
            }
            0xE0 => {
                // LDH (a8),A
                let offset = self.fetch();
                let addr = 0xFF00 | (offset as u16);
                self.mmu.write_byte(addr, self.registers.a);
                12
            }
            0xF0 => {
                // LDH A, (a8) - 從高位記憶體讀取到 A
                let offset = self.fetch();
                let addr = 0xFF00 | (offset as u16);
                self.registers.a = self.mmu.read_byte(addr);
                12
            }
            // === 8-bit 載入指令 ===
            0x06 => {
                // LD B, n
                let n = self.fetch();
                self.registers.b = n;
                8
            }
            0x0E => {
                // LD C, n
                let n = self.fetch();
                self.registers.c = n;
                8
            }
            0x16 => {
                // LD D, n
                let n = self.fetch();
                self.registers.d = n;
                8
            }
            0x1E => {
                // LD E, n
                let n = self.fetch();
                self.registers.e = n;
                8
            }
            0x26 => {
                // LD H, n
                let n = self.fetch();
                self.registers.h = n;
                8
            }
            0x2E => {
                // LD L, n
                let n = self.fetch();
                self.registers.l = n;
                8
            }
            0x3E => {
                // LD A, n
                let n = self.fetch();
                self.registers.a = n;
                8
            }
            0x32 => self.execute_load_instruction(0x32), // LD (HL-),A
            0x33 => {
                // INC SP - 堆疊指針加一
                self.registers.sp = self.registers.sp.wrapping_add(1);
                8
            }
            0x36 => {
                // LD (HL), n
                let n = self.fetch();
                let addr = self.registers.get_hl();
                self.mmu.write_byte(addr, n);
                12
            }

            // 寄存器間載入
            0x40 => 4, // LD B, B (無需實際操作)
            0x41 => {
                self.registers.b = self.registers.c;
                4
            } // LD B, C
            0x42 => {
                self.registers.b = self.registers.d;
                4
            } // LD B, D
            0x43 => {
                self.registers.b = self.registers.e;
                4
            } // LD B, E
            0x44 => {
                self.registers.b = self.registers.h;
                4
            } // LD B, H
            0x45 => {
                self.registers.b = self.registers.l;
                4
            } // LD B, L
            0x46 => {
                // LD B, (HL)
                let addr = self.registers.get_hl();
                self.registers.b = self.mmu.read_byte(addr);
                8
            }
            0x47 => {
                self.registers.b = self.registers.a;
                4
            } // LD B, A

            0x48 => {
                self.registers.c = self.registers.b;
                4
            } // LD C, B
            0x49 => 4, // LD C, C (無需實際操作)
            0x4A => {
                self.registers.c = self.registers.d;
                4
            } // LD C, D
            0x4B => {
                self.registers.c = self.registers.e;
                4
            } // LD C, E
            0x4C => {
                self.registers.c = self.registers.h;
                4
            } // LD C, H
            0x4D => {
                self.registers.c = self.registers.l;
                4
            } // LD C, L
            0x4E => {
                // LD C, (HL)
                let addr = self.registers.get_hl();
                self.registers.c = self.mmu.read_byte(addr);
                8
            }
            0x4F => {
                self.registers.c = self.registers.a;
                4
            } // LD C, A

            0x78 => {
                self.registers.a = self.registers.b;
                4
            } // LD A, B
            0x79 => {
                self.registers.a = self.registers.c;
                4
            } // LD A, C
            0x7A => {
                self.registers.a = self.registers.d;
                4
            } // LD A, D
            0x7B => {
                self.registers.a = self.registers.e;
                4
            } // LD A, E
            0x7C => {
                self.registers.a = self.registers.h;
                4
            } // LD A, H
            0x7D => {
                self.registers.a = self.registers.l;
                4
            } // LD A, L
            0x7E => {
                // LD A, (HL)
                let addr = self.registers.get_hl();
                self.registers.a = self.mmu.read_byte(addr);
                8
            }
            0x7F => 4, // LD A, A (無動作)

            0x77 => {
                // LD (HL), A
                let addr = self.registers.get_hl();
                self.mmu.write_byte(addr, self.registers.a);
                8
            }

            // === 16-bit 載入指令 ===
            0x01 => {
                // LD BC, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.set_bc((hi as u16) << 8 | lo as u16);
                12
            }
            0x11 => {
                // LD DE, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.set_de((hi as u16) << 8 | lo as u16);
                12
            }
            0x21 => {
                // LD HL, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.set_hl((hi as u16) << 8 | lo as u16);
                12
            }
            0x22 => {
                // LD (HL+), A - 將 A 存入 HL 指向的地址，然後 HL 加一
                let addr = self.registers.get_hl();
                self.mmu.write_byte(addr, self.registers.a);

                // HL++
                let hl = self.registers.get_hl().wrapping_add(1);
                self.registers.set_hl(hl);

                8
            }
            0x23 => {
                // INC HL - HL 寄存器對加一
                let hl = self.registers.get_hl().wrapping_add(1);
                self.registers.set_hl(hl);
                8
            }
            0x2A => {
                // LD A, (HL+) - 讀取 HL 指向的地址到 A，然後 HL 加一
                let addr = self.registers.get_hl();
                self.registers.a = self.mmu.read_byte(addr);

                // HL++
                let hl = self.registers.get_hl().wrapping_add(1);
                self.registers.set_hl(hl);

                8
            }
            0x31 => {
                // LD SP, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.sp = (hi as u16) << 8 | lo as u16;
                12
            }

            // === 算術指令 ===
            0x04 => {
                // INC B
                self.registers.b = self.registers.b.wrapping_add(1);
                self.registers.set_z_flag(self.registers.b == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.b & 0x0F) == 0);
                4
            }
            0x05 => {
                // DEC B - B寄存器減一
                let value = self.registers.b.wrapping_sub(1);
                self.registers.set_z_flag(value == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.b & 0x0F) == 0x00);
                self.registers.b = value;
                4
            }
            0x0C => {
                // INC C
                self.registers.c = self.registers.c.wrapping_add(1);
                self.registers.set_z_flag(self.registers.c == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.c & 0x0F) == 0);
                4
            }
            0x0D => {
                // DEC C - C寄存器減一
                self.registers.c = self.registers.c.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.c == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.c & 0x0F) == 0x0F);
                4
            }
            0x3C => {
                // INC A
                self.registers.a = self.registers.a.wrapping_add(1);
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.a & 0x0F) == 0);
                4
            }
            0x80..=0x87 => {
                // ADD A,r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                let (result, carry) = self.registers.a.overflowing_add(value);
                let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F;

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(half_carry);
                self.registers.set_c_flag(carry);

                self.registers.a = result;
                if reg_num == 6 { 8 } else { 4 }
            }
            0x88..=0x8F => {
                // ADC A,r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                let carry = if self.registers.get_c_flag() { 1 } else { 0 };
                let result = self.registers.a.wrapping_add(value).wrapping_add(carry);
                let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) + carry > 0x0F;
                let full_carry = (self.registers.a as u16) + (value as u16) + (carry as u16) > 0xFF;

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(half_carry);
                self.registers.set_c_flag(full_carry);

                self.registers.a = result;
                if reg_num == 6 { 8 } else { 4 }
            }
            0x90..=0x97 => {
                // SUB r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                let (result, carry) = self.registers.a.overflowing_sub(value);
                let half_carry = (self.registers.a & 0x0F) < (value & 0x0F);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag(half_carry);
                self.registers.set_c_flag(carry);

                self.registers.a = result;
                if reg_num == 6 { 8 } else { 4 }
            }
            0x98..=0x9F => {
                // SBC A,r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                let carry = if self.registers.get_c_flag() { 1 } else { 0 };
                let result = self.registers.a.wrapping_sub(value).wrapping_sub(carry);
                let half_carry = (self.registers.a & 0x0F) < (value & 0x0F) + carry;
                let full_carry = (self.registers.a as i16) - (value as i16) - (carry as i16) < 0;

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag(half_carry);
                self.registers.set_c_flag(full_carry);

                self.registers.a = result;
                if reg_num == 6 { 8 } else { 4 }
            }
            0xA0..=0xA7 => {
                // AND r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                self.registers.a &= value;

                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(true);
                self.registers.set_c_flag(false);

                if reg_num == 6 { 8 } else { 4 }
            }
            0xB0..=0xB7 => {
                // OR r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                self.registers.a |= value;

                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);

                if reg_num == 6 { 8 } else { 4 }
            }
            0xA8..=0xAF => {
                // XOR r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                self.registers.a ^= value;

                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);

                if reg_num == 6 { 8 } else { 4 }
            }
            0xB8..=0xBF => {
                // CP r
                let reg_num = opcode & 0x07;
                let value = self.get_register_8bit(reg_num);
                let result = self.registers.a.wrapping_sub(value);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers
                    .set_h_flag((self.registers.a & 0x0F) < (value & 0x0F));
                self.registers.set_c_flag(self.registers.a < value);

                if reg_num == 6 { 8 } else { 4 }
            }
            0xFE => {
                // CP n - Compare A with immediate value
                let n = self.fetch();
                let result = self.registers.a.wrapping_sub(n);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers
                    .set_h_flag((self.registers.a & 0x0F) < (n & 0x0F));
                self.registers.set_c_flag(self.registers.a < n);

                8 // 2 機器週期
            }
            0x2F => {
                // CPL - 累加器取補碼 (NOT A)
                self.registers.a = !self.registers.a; // 全部位元取反
                self.registers.set_n_flag(true);
                self.registers.set_h_flag(true);
                4
            }
            0x27 => {
                // DAA - 十進制調整累加器 (用於 BCD 算術後的調整)
                let mut a = self.registers.a;
                let mut adjust = 0;

                if !self.registers.get_n_flag() {
                    // 加法操作後
                    if self.registers.get_h_flag() || (a & 0x0F) > 0x09 {
                        adjust |= 0x06;
                    }
                    if self.registers.get_c_flag() || a > 0x99 {
                        adjust |= 0x60;
                        self.registers.set_c_flag(true);
                    }
                    a = a.wrapping_add(adjust);
                } else {
                    // 減法操作後
                    if self.registers.get_h_flag() {
                        adjust |= 0x06;
                    }
                    if self.registers.get_c_flag() {
                        adjust |= 0x60;
                    }
                    a = a.wrapping_sub(adjust);
                }

                self.registers.a = a;
                self.registers.set_z_flag(a == 0);
                self.registers.set_h_flag(false);
                // C 標誌在加法時已更新,減法時保持不變
                4
            } // === 16 位元算術和跳轉指令 ===
            0x00 => 4, // NOP
            0x03 => {
                // INC BC - BC 寄存器對加一
                let bc = self.registers.get_bc().wrapping_add(1);
                self.registers.set_bc(bc);
                8
            }
            0x09 => {
                // ADD HL, BC - HL 加上 BC
                let hl = self.registers.get_hl();
                let bc = self.registers.get_bc();
                let result = hl.wrapping_add(bc);

                // 不影響零標誌
                self.registers.set_n_flag(false);
                // 半進位：檢查第11位的進位（相當於16位數的低位8位的進位）
                self.registers
                    .set_h_flag((hl & 0xFFF) + (bc & 0xFFF) > 0xFFF);
                // 進位：檢查結果是否溢出16位
                self.registers.set_c_flag(hl > 0xFFFF - bc);

                self.registers.set_hl(result);
                8
            }
            0x0B => {
                // DEC BC - BC 寄存器對減一
                let bc = self.registers.get_bc().wrapping_sub(1);
                self.registers.set_bc(bc);
                8
            }
            0x13 => {
                // INC DE - DE 寄存器對加一
                let de = self.registers.get_de().wrapping_add(1);
                self.registers.set_de(de);
                8
            }
            0x18 => {
                // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                12
            }
            0x19 => {
                // ADD HL, DE - HL 加上 DE
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                let result = hl.wrapping_add(de);

                // 不影響零標誌
                self.registers.set_n_flag(false);
                // 半進位：檢查第11位的進位（相當於16位數的低位8位的進位）
                self.registers
                    .set_h_flag((hl & 0xFFF) + (de & 0xFFF) > 0xFFF);
                // 進位：檢查結果是否溢出16位
                self.registers.set_c_flag(hl > 0xFFFF - de);

                self.registers.set_hl(result);
                8
            }
            0x1B => {
                // DEC DE - DE 寄存器對減一
                let de = self.registers.get_de().wrapping_sub(1);
                self.registers.set_de(de);
                8
            }
            0x20 => {
                // JR NZ, n
                let offset = self.fetch() as i8;
                if !self.registers.get_z_flag() {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                    12
                } else {
                    8
                }
            }
            0x28 => {
                // JR Z, n
                let offset = self.fetch() as i8;
                if self.registers.get_z_flag() {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                    12
                } else {
                    8
                }
            }
            0x30 => {
                // JR NC, n - 如果進位標誌為 0 則跳轉
                let offset = self.fetch() as i8;
                if !self.registers.get_c_flag() {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                    12
                } else {
                    8
                }
            }
            0x38 => {
                // JR C, n - 如果進位標誌為 1 則跳轉
                let offset = self.fetch() as i8;
                if self.registers.get_c_flag() {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                    12
                } else {
                    8
                }
            }
            0x29 => {
                // ADD HL, HL - HL 加上 HL
                let hl = self.registers.get_hl();
                let result = hl.wrapping_add(hl);

                // 不影響零標誌
                self.registers.set_n_flag(false);
                // 半進位：檢查第11位的進位
                self.registers
                    .set_h_flag((hl & 0xFFF) + (hl & 0xFFF) > 0xFFF);
                // 進位：檢查結果是否溢出16位
                self.registers.set_c_flag(hl > 0x7FFF); // 0xFFFF/2

                self.registers.set_hl(result);
                8
            }
            0x2B => {
                // DEC HL - HL 寄存器對減一
                let hl = self.registers.get_hl().wrapping_sub(1);
                self.registers.set_hl(hl);
                8
            }
            0x39 => {
                // ADD HL, SP - HL 加上 SP
                let hl = self.registers.get_hl();
                let sp = self.registers.sp;
                let result = hl.wrapping_add(sp);

                // 不影響零標誌
                self.registers.set_n_flag(false);
                // 半進位：檢查第11位的進位
                self.registers
                    .set_h_flag((hl & 0xFFF) + (sp & 0xFFF) > 0xFFF);
                // 進位：檢查結果是否溢出16位
                self.registers.set_c_flag(hl > 0xFFFF - sp);

                self.registers.set_hl(result);
                8
            }
            0x3B => {
                // DEC SP - 堆疊指針減一
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                8
            }
            0xC3 => {
                // JP nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
                16
            }
            0xE9 => {
                // JP (HL)
                self.registers.pc = self.registers.get_hl();
                4
            }

            // === CPU 控制指令 ===
            0x76 => {
                // HALT
                if self.ime {
                    self.state = CPUState::Halted;
                } else {
                    // HALT bug: 當 IME=0 且有未處理的中斷時，
                    // 下一條指令會被執行兩次
                    let if_reg = self.mmu.read_byte(0xFF0F);
                    let ie_reg = self.mmu.read_byte(0xFFFF);
                    if if_reg & ie_reg & 0x1F != 0 {
                        self.halt_bug = true;
                    } else {
                        self.state = CPUState::Halted;
                    }
                }
                4
            }
            0x10 => {
                // STOP
                let _ = self.fetch(); // STOP 的第二個字節總是 0x00
                self.state = CPUState::Stopped;
                4
            }
            0xF3 => {
                // DI
                self.ime = false;
                4
            }
            0xFB => {
                // EI
                self.ei_delay = true; // EI 有一個指令的延遲
                4
            }

            // === CB 前綴指令 ===
            0xCB => {
                let cb_opcode = self.fetch();
                self.execute_cb_instruction(cb_opcode)
            }

            // === 堆疊和返回指令 ===
            0xC1 => {
                // POP BC
                let value = self.pop_stack();
                self.registers.set_bc(value);
                12
            }
            0xD1 => {
                // POP DE
                let value = self.pop_stack();
                self.registers.set_de(value);
                12
            }
            0xE1 => {
                // POP HL
                let value = self.pop_stack();
                self.registers.set_hl(value);
                12
            }
            0xF1 => {
                // POP AF
                let value = self.pop_stack();
                self.registers.set_af(value);
                12
            }
            0xC5 => {
                // PUSH BC
                self.push_stack(self.registers.get_bc());
                16
            }
            0xD5 => {
                // PUSH DE
                let de = self.registers.get_de();
                self.push_stack(de);
                16
            }
            0xE5 => {
                // PUSH HL
                self.push_stack(self.registers.get_hl());
                16
            }
            0xF5 => {
                // PUSH AF
                self.push_stack(self.registers.get_af());
                16
            }
            0xC9 => {
                // RET
                self.registers.pc = self.pop_stack();
                16
            }
            0xCD => {
                // CALL nn - 呼叫子程序
                let low = self.fetch();
                let high = self.fetch();
                let address = ((high as u16) << 8) | (low as u16);

                // 將下一條指令的地址（PC）壓入堆疊
                self.push_stack(self.registers.pc);

                // 跳轉到目標地址
                self.registers.pc = address;

                24 // 需要 24 個 T-states (6 機器週期)
            }
            0xC4 => {
                // CALL NZ, nn - 如果 Z 標誌為 0（非零）則調用
                let low = self.fetch();
                let high = self.fetch();
                let address = ((high as u16) << 8) | (low as u16);

                if !self.registers.get_z_flag() {
                    // 將下一條指令的地址（PC）壓入堆疊
                    self.push_stack(self.registers.pc);

                    // 跳轉到目標地址
                    self.registers.pc = address;
                    24 // 條件成立時需要 24 個 T-states
                } else {
                    12 // 條件不成立時只需要 12 個 T-states
                }
            }
            0xCC => {
                // CALL Z, nn - 如果 Z 標誌為 1（零）則調用
                let low = self.fetch();
                let high = self.fetch();
                let address = ((high as u16) << 8) | (low as u16);

                if self.registers.get_z_flag() {
                    // 將下一條指令的地址（PC）壓入堆疊
                    self.push_stack(self.registers.pc);

                    // 跳轉到目標地址
                    self.registers.pc = address;
                    24 // 條件成立時需要 24 個 T-states
                } else {
                    12 // 條件不成立時只需要 12 個 T-states
                }
            }
            0xC0 => {
                // RET NZ
                if !self.registers.get_z_flag() {
                    self.registers.pc = self.pop_stack();
                    20
                } else {
                    8
                }
            }
            0xD4 => {
                // CALL NC, nn - 如果 C 標誌為 0（無進位）則調用
                let low = self.fetch();
                let high = self.fetch();
                let address = ((high as u16) << 8) | (low as u16);

                if !self.registers.get_c_flag() {
                    // 將下一條指令的地址（PC）壓入堆疊
                    self.push_stack(self.registers.pc);

                    // 跳轉到目標地址
                    self.registers.pc = address;
                    24 // 條件成立時需要 24 個 T-states
                } else {
                    12 // 條件不成立時只需要 12 個 T-states
                }
            }
            0xDC => {
                // CALL C, nn - 如果 C 標誌為 1（有進位）則調用
                let low = self.fetch();
                let high = self.fetch();
                let address = ((high as u16) << 8) | (low as u16);

                if self.registers.get_c_flag() {
                    // 將下一條指令的地址（PC）壓入堆疊
                    self.push_stack(self.registers.pc);

                    // 跳轉到目標地址
                    self.registers.pc = address;
                    24 // 條件成立時需要 24 個 T-states
                } else {
                    12 // 條件不成立時只需要 12 個 T-states
                }
            }
            0xC8 => {
                // RET Z
                if self.registers.get_z_flag() {
                    self.registers.pc = self.pop_stack();
                    20
                } else {
                    8
                }
            }
            0xD0 => {
                // RET NC
                if !self.registers.get_c_flag() {
                    self.registers.pc = self.pop_stack();
                    20
                } else {
                    8
                }
            }
            0xD8 => {
                // RET C
                if self.registers.get_c_flag() {
                    self.registers.pc = self.pop_stack();
                    20
                } else {
                    8
                }

                4
            }
            0xE6 => {
                // AND n - A 與立即數進行 AND 運算
                let n = self.fetch();
                self.registers.a &= n;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(true);
                self.registers.set_c_flag(false);
                8
            }
        }
    }

    // CB 前綴指令處理（位操作和旋轉指令）
    fn execute_cb_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // RLC/RRC/RL/RR r - 旋轉指令
            0x00..=0x07 => {
                // RLC r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let carry = (value & 0x80) != 0;
                let result = (value << 1) | (if carry { 1 } else { 0 });

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }
            0x08..=0x0F => {
                // RRC r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let carry = (value & 0x01) != 0;
                let result = (value >> 1) | (if carry { 0x80 } else { 0 });

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }
            0x10..=0x17 => {
                // RL r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let old_carry = self.registers.get_c_flag();
                let new_carry = (value & 0x80) != 0;
                let result = (value << 1) | (if old_carry { 1 } else { 0 });

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(new_carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }
            0x18..=0x1F => {
                // RR r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let old_carry = self.registers.get_c_flag();
                let new_carry = (value & 0x01) != 0;
                let result = (value >> 1) | (if old_carry { 0x80 } else { 0 });

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(new_carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }

            // SLA/SWAP/SRA/SRL r - 移位指令
            0x20..=0x27 => {
                // SLA r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let carry = (value & 0x80) != 0;
                let result = value << 1;

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }
            0x30..=0x37 => {
                // SWAP r - 交換高低4位
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let hi = value & 0xF0;
                let lo = value & 0x0F;
                let result = (lo << 4) | (hi >> 4);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }
            0x28..=0x2F => {
                // SRA r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let msb = value & 0x80;
                let carry = (value & 0x01) != 0;
                let result = (value >> 1) | msb;

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }
            0x38..=0x3F => {
                // SRL r
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let carry = (value & 0x01) != 0;
                let result = value >> 1;

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }

            // BIT b,r - 位測試指令
            0x40..=0x7F => {
                let bit = (opcode - 0x40) >> 3;
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);

                self.registers.set_z_flag((value & (1 << bit)) == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(true);

                if reg == 6 { 12 } else { 8 }
            } // SET b,r - 位設置指令（不影響任何旗標）
            0xC0..=0xFF => {
                let bit = (opcode - 0xC0) >> 3;
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let result = value | (1 << bit);
                // SET 指令不影響任何旗標
                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }

            // RES b,r - 位重置指令（不影響任何旗標）
            0x80..=0xBF => {
                let bit = (opcode - 0x80) >> 3;
                let reg = opcode & 0x07;
                let value = self.get_register_8bit(reg);
                let result = value & !(1 << bit);
                // RES 指令不影響任何旗標

                self.set_register_8bit(reg, result);
                if reg == 6 { 16 } else { 8 }
            }

            // 其他未實現的 CB 指令
            n => {
                println!("警告：未實現的 CB 指令 0x{:02X}", n);
                8
            }
        }
    } // 必需的方法
    pub fn get_enhanced_status_report(&self) -> String {
        format!(
            "===== CPU Status Report =====\n\
             Program Counter (PC): 0x{:04X}\n\
             Stack Pointer (SP): 0x{:04X}\n\
             \n\
             Registers:\n\
             A (Accumulator): 0x{:02X}   F (Flags): 0x{:02X}\n\
             B: 0x{:02X}                 C: 0x{:02X}\n\
             D: 0x{:02X}                 E: 0x{:02X}\n\
             H: 0x{:02X}                 L: 0x{:02X}\n\
             \n\
             16-bit Register Pairs:\n\
             AF: 0x{:04X}                BC: 0x{:04X}\n\
             DE: 0x{:04X}                HL: 0x{:04X}\n\
             \n\
             CPU State: {:?}\n\
             Interrupt Master Enable (IME): {}\n\
             \n\
             Flags:\n\
             Zero (Z): {}                Subtract (N): {}\n\
             Half Carry (H): {}          Carry (C): {}\n\
             \n\
             Performance:\n\
             Total Instructions: {}\n\
             Total Machine Cycles: {}\n\
             Total Clock Cycles: {}\n\
             \n\
             Memory at PC:\n\
             Next bytes: {:02X} {:02X} {:02X} {:02X}",
            self.registers.pc,
            self.registers.sp,
            self.registers.a,
            self.registers.f,
            self.registers.b,
            self.registers.c,
            self.registers.d,
            self.registers.e,
            self.registers.h,
            self.registers.l,
            self.registers.get_af(),
            self.registers.get_bc(),
            self.registers.get_de(),
            self.registers.get_hl(),
            self.state,
            self.ime,
            self.registers.get_z_flag(),
            self.registers.get_n_flag(),
            self.registers.get_h_flag(),
            self.registers.get_c_flag(),
            self.instruction_count,
            self.total_cycles,
            self.total_cycles * 4,
            self.mmu.read_byte(self.registers.pc),
            self.mmu.read_byte(self.registers.pc.wrapping_add(1)),
            self.mmu.read_byte(self.registers.pc.wrapping_add(2)),
            self.mmu.read_byte(self.registers.pc.wrapping_add(3))
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
        self.state == CPUState::Halted
    }

    pub fn get_instruction_count(&self) -> u64 {
        self.instruction_count
    }

    pub fn get_total_cycles(&self) -> u64 {
        self.total_cycles
    }

    pub fn save_performance_report(&self) {
        let report = format!(
            "Performance Report:\n\
             Total Instructions: {}\n\
             Total Cycles: {}\n\
             PC: 0x{:04X}\n\
             State: {:?}\n\
             IME: {}\n\
             Registers: A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} H={:02X} L={:02X}\n\
             Flags: Z:{} N:{} H:{} C:{}\n",
            self.instruction_count,
            self.total_cycles,
            self.registers.pc,
            self.state,
            self.ime,
            self.registers.a,
            self.registers.b,
            self.registers.c,
            self.registers.d,
            self.registers.e,
            self.registers.h,
            self.registers.l,
            self.registers.get_z_flag(),
            self.registers.get_n_flag(),
            self.registers.get_h_flag(),
            self.registers.get_c_flag()
        );

        if let Ok(mut file) = std::fs::File::create("debug_report/performance_report.txt") {
            use std::io::Write;
            let _ = file.write_all(report.as_bytes());
        }
    }

    // CPU 狀態輔助方法
    pub fn is_halted(&self) -> bool {
        self.state == CPUState::Halted
    }

    pub fn is_stopped(&self) -> bool {
        self.state == CPUState::Stopped
    }

    pub fn is_ime_enabled(&self) -> bool {
        self.ime
    }

    // 硬體時序模擬
    pub fn tick(&mut self, cycles: u8) {
        self.total_cycles += cycles as u64;

        // 更新 LCD 掃描線計數器
        let ly_cycles = self.total_cycles % 456;
        if ly_cycles == 0 {
            let ly_addr = 0xFF44;
            let current_ly = self.mmu.read_byte(ly_addr);

            if current_ly >= 153 {
                self.mmu.write_byte(ly_addr, 0);
            } else {
                self.mmu.write_byte(ly_addr, current_ly + 1);
            }

            // V-Blank 中斷
            if current_ly == 144 {
                let if_reg = self.mmu.read_byte(0xFF0F);
                self.mmu.write_byte(0xFF0F, if_reg | VBLANK_FLAG);
            }
        }
    }

    // 中斷處理相關方法
    pub fn request_interrupt(&mut self, interrupt: u8) {
        let if_reg = self.mmu.read_byte(0xFF0F);
        self.mmu.write_byte(0xFF0F, if_reg | interrupt);
    }

    pub fn clear_interrupt(&mut self, interrupt: u8) {
        let if_reg = self.mmu.read_byte(0xFF0F);
        self.mmu.write_byte(0xFF0F, if_reg & !interrupt);
    }

    // 調試輔助方法
    pub fn print_next_instruction(&self) {
        let pc = self.registers.pc;
        let opcode = self.mmu.read_byte(pc);

        print!("PC: 0x{:04X} - ", pc);

        if opcode == 0xCB {
            let cb_opcode = self.mmu.read_byte(pc + 1);
            println!("CB {:02X}", cb_opcode);
        } else {
            println!("{:02X}", opcode);
        }
    }

    pub fn get_register_state(&self) -> String {
        format!(
            "A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: {:04X}",
            self.registers.a,
            self.registers.f,
            self.registers.b,
            self.registers.c,
            self.registers.d,
            self.registers.e,
            self.registers.h,
            self.registers.l,
            self.registers.sp,
            self.registers.pc
        )
    }

    // 保存調試報告
    pub fn save_debug_report(&self, filename: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(filename)?;

        writeln!(file, "{}", self.get_enhanced_status_report())?;
        writeln!(file, "\nMemory Map:")?;

        // 輸出關鍵內存區域的內容
        for addr in (0..0x100).step_by(16) {
            write!(file, "\n{:04X}:", addr)?;
            for offset in 0..16 {
                write!(file, " {:02X}", self.mmu.read_byte(addr + offset))?;
            }
        }

        Ok(())
    }

    // 輔助方法：獲取 8 位寄存器值
    fn get_register_8bit(&mut self, reg: u8) -> u8 {
        match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => 0,
        }
    }

    // 輔助方法：設置 8 位寄存器值
    fn set_register_8bit(&mut self, reg: u8, value: u8) {
        match reg {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => self.mmu.write_byte(self.registers.get_hl(), value),
            7 => self.registers.a = value,
            _ => {}
        }
    }

    // 新增: 記錄VRAM寫入
    fn record_vram_write(&mut self, addr: u16, value: u8) {
        if self.vram_writes.len() >= 100 {
            self.vram_writes.remove(0);
        }
        self.vram_writes.push((addr, value, self.registers.pc));
    }

    // 獲取最近的追蹤信息
    pub fn get_recent_trace(&self) -> String {
        let mut output = String::new();
        if let Some(last_traces) = self
            .execution_trace
            .get(self.execution_trace.len().saturating_sub(10)..)
        {
            for trace in last_traces {
                output.push_str(&format!(
                    "PC:{:04X} OP:{:02X} A:{:02X} F:{:02X} BC:{:02X}{:02X} DE:{:02X}{:02X} HL:{:02X}{:02X} SP:{:04X} CYC:{}\n",
                    trace.pc,
                    trace.opcode,
                    trace.registers.a,
                    trace.registers.f,
                    trace.registers.b,
                    trace.registers.c,
                    trace.registers.d,
                    trace.registers.e,
                    trace.registers.h,
                    trace.registers.l,
                    trace.registers.sp,
                    trace.cycles
                ));
            }
        }
        output
    }

    // 獲取 VRAM 寫入歷史
    pub fn get_vram_writes(&self) -> String {
        let mut output = String::new();
        for &(addr, value, pc) in self.vram_writes.iter().rev().take(10) {
            output.push_str(&format!(
                "PC:{:04X} 寫入 VRAM[{:04X}]={:02X}\n",
                pc, addr, value
            ));
        }
        output
    }

    fn debug_instruction(&self, opcode: u8) {
        // 獲取下一個字節（如果需要）
        let next_byte = self.mmu.read_byte(self.registers.pc + 1);
        let next_word = self.mmu.read_word(self.registers.pc + 1);

        println!("\n=== 指令執行 ===");
        println!("PC: 0x{:04X}  指令: 0x{:02X}", self.registers.pc, opcode);
        println!("下一字節: 0x{:02X}  下一字: 0x{:04X}", next_byte, next_word);
        println!("寄存器狀態:");
        println!(
            "  A: 0x{:02X}  F: 0x{:02X} [Z:{}{}{}{}]",
            self.registers.a,
            self.registers.f,
            if self.registers.get_z_flag() {
                "1"
            } else {
                "0"
            },
            if self.registers.get_n_flag() {
                "1"
            } else {
                "0"
            },
            if self.registers.get_h_flag() {
                "1"
            } else {
                "0"
            },
            if self.registers.get_c_flag() {
                "1"
            } else {
                "0"
            }
        );
        println!(
            "  B: 0x{:02X}  C: 0x{:02X}  BC: 0x{:04X}",
            self.registers.b,
            self.registers.c,
            self.registers.get_bc()
        );
        println!(
            "  D: 0x{:02X}  E: 0x{:02X}  DE: 0x{:04X}",
            self.registers.d,
            self.registers.e,
            self.registers.get_de()
        );
        println!(
            "  H: 0x{:02X}  L: 0x{:02X}  HL: 0x{:04X}",
            self.registers.h,
            self.registers.l,
            self.registers.get_hl()
        );
        println!("  SP: 0x{:04X}", self.registers.sp);
    }
}
