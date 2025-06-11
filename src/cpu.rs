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
    // 中斷控制
    pub ime: bool,    // 中斷主開關 (Interrupt Master Enable)
    pub halted: bool, // CPU 是否處於 HALT 狀態
}

impl CPU {
    // --- ALU 運算相關方法 ---
    fn alu_add(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_add(value);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(((a & 0xF) + (value & 0xF)) > 0xF);
        self.registers.set_c_flag((a as u16 + value as u16) > 0xFF);
        self.registers.a = result;
    }
    fn alu_adc(&mut self, value: u8) {
        let a = self.registers.a;
        let c = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
        let result = a.wrapping_add(value).wrapping_add(c);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers
            .set_h_flag(((a & 0xF) + (value & 0xF) + c) > 0xF);
        self.registers
            .set_c_flag((a as u16 + value as u16 + c as u16) > 0xFF);
        self.registers.a = result;
    }
    fn alu_sub(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((a & 0xF) < (value & 0xF));
        self.registers.set_c_flag(a < value);
        self.registers.a = result;
    }
    fn alu_sbc(&mut self, value: u8) {
        let a = self.registers.a;
        let c = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
        let result = a.wrapping_sub(value).wrapping_sub(c);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((a & 0xF) < ((value & 0xF) + c));
        self.registers
            .set_c_flag((a as u16) < (value as u16 + c as u16));
        self.registers.a = result;
    }
    fn alu_and(&mut self, value: u8) {
        let result = self.registers.a & value;
        self.registers.a = result;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(true);
        self.registers.set_c_flag(false);
    }
    fn alu_or(&mut self, value: u8) {
        let result = self.registers.a | value;
        self.registers.a = result;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);
    }
    fn alu_xor(&mut self, value: u8) {
        let result = self.registers.a ^ value;
        self.registers.a = result;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);
    }
    fn alu_cp(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((a & 0xF) < (value & 0xF));
        self.registers.set_c_flag(a < value);
    }

    // --- 位元操作指令 ---
    fn rlc(&mut self, value: u8) -> u8 {
        let result = (value << 1) | (value >> 7);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x80) != 0);
        result
    }
    fn rrc(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | ((value & 0x01) << 7);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }
    fn rl(&mut self, value: u8) -> u8 {
        let c = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
        let result = (value << 1) | c;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x80) != 0);
        result
    }
    fn rr(&mut self, value: u8) -> u8 {
        let c = if (self.registers.f & 0x10) != 0 {
            0x80
        } else {
            0
        };
        let result = (value >> 1) | c;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }
    fn sla(&mut self, value: u8) -> u8 {
        let result = value << 1;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x80) != 0);
        result
    }
    fn sra(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | (value & 0x80);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }
    fn srl(&mut self, value: u8) -> u8 {
        let result = value >> 1;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }
    fn swap(&mut self, value: u8) -> u8 {
        let result = (value >> 4) | (value << 4);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);
        result
    }

    // --- 其他公共方法 ---
    pub fn new(mmu: MMU) -> Self {
        let mut registers = Registers::default();
        registers.pc = 0x0100; // Game Boy CPU 應該從 0x0100 開始執行
        registers.sp = 0xFFFE; // 初始化堆疊指標

        CPU {
            registers,
            mmu,
            instruction_count: 0,
            ime: false,    // 中斷主開關初始化為關閉
            halted: false, // CPU 初始狀態為未暫停
        }
    }
    pub fn step(&mut self) {
        // 首先處理中斷
        self.handle_interrupts();
        // 然後執行指令
        self.execute();
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
        // 檢查 PC 是否指向非法地址
        if self.registers.pc >= 0xFF00 {
            println!(
                "🚨 警告：CPU 嘗試從非法地址 0x{:04X} 讀取指令！",
                self.registers.pc
            );

            // 如果 PC 指向 I/O 區域或中斷向量，這是不正常的
            // 強制跳轉到安全位置
            if self.registers.pc == 0xFFFF {
                println!("💀 致命錯誤：PC 指向 IE 寄存器 (0xFFFF)");
                println!("🔧 自動修復：重置到 ROM 入口點");
                self.registers.pc = 0x0100; // Game Boy ROM 入口點
                self.registers.sp = 0xFFFE; // 重置堆疊指針
            } else if self.registers.pc >= 0xFF80 && self.registers.pc <= 0xFFFE {
                println!(
                    "💀 致命錯誤：PC 指向 HRAM 區域 (0x{:04X})",
                    self.registers.pc
                );
                println!("🔧 自動修復：重置到 ROM 入口點");
                self.registers.pc = 0x0100;
                self.registers.sp = 0xFFFE;
            } else {
                println!(
                    "💀 致命錯誤：PC 指向 I/O 區域 (0x{:04X})",
                    self.registers.pc
                );
                println!("🔧 自動修復：重置到 ROM 入口點");
                self.registers.pc = 0x0100;
                self.registers.sp = 0xFFFE;
            }
        }

        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        opcode
    }

    fn decode_and_execute(&mut self, opcode: u8) {
        // 添加調試輸出來追蹤指令執行
        if self.instruction_count < 20 {
            println!(
                "執行指令: PC=0x{:04X}, opcode=0x{:02X}",
                self.registers.pc.wrapping_sub(1),
                opcode
            );
        }

        match opcode {
            // 基本 CPU 控制指令
            0x00 => {} // NOP
            0x76 => {} // HALT

            // 跳轉指令
            0x18 => {
                // JR n (相對跳轉)
                let offset = self.fetch() as i8;
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
            }
            0x20 => {
                // JR NZ, n (如果 Z 標誌未設置則相對跳轉)
                let offset = self.fetch() as i8;
                let z_flag = (self.registers.f & 0x80) != 0;
                if !z_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0x28 => {
                // JR Z, n (如果 Z 標誌設置則相對跳轉)
                let offset = self.fetch() as i8;
                let z_flag = (self.registers.f & 0x80) != 0;
                if z_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0x30 => {
                // JR NC, n (如果 C 標誌未設置則相對跳轉)
                let offset = self.fetch() as i8;
                let c_flag = (self.registers.f & 0x10) != 0;
                if !c_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0x38 => {
                // JR C, n (如果 C 標誌設置則相對跳轉)
                let offset = self.fetch() as i8;
                let c_flag = (self.registers.f & 0x10) != 0;
                if c_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0xC3 => {
                // JP nn (絕對跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xC9 => {
                // RET (從棧中彈出地址並跳轉)
                let sp = self.registers.sp;
                let lo = self.mmu.read_byte(sp) as u16;
                let hi = self.mmu.read_byte(sp + 1) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.pc = (hi << 8) | lo;
            }
            0xDA => {
                // JP C, nn (如果 C 標誌設置則跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let c_flag = (self.registers.f & 0x10) != 0;
                if c_flag {
                    self.registers.pc = addr;
                }
            }

            // 8-bit 加載指令
            0x06 => {
                // LD B, n
                let n = self.fetch();
                self.registers.b = n;
            }
            0x0E => {
                // LD C, n
                let n = self.fetch();
                self.registers.c = n;
            }
            0x16 => {
                // LD D, n
                let n = self.fetch();
                self.registers.d = n;
            }
            0x1E => {
                // LD E, n
                let n = self.fetch();
                self.registers.e = n;
            }
            0x26 => {
                // LD H, n
                let n = self.fetch();
                self.registers.h = n;
            }
            0x2E => {
                // LD L, n
                let n = self.fetch();
                self.registers.l = n;
            }
            0x3E => {
                // LD A, n
                let n = self.fetch();
                self.registers.a = n;
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
            0xF0 => {
                // LDH A, (n) (從 0xFF00+n 載入到 A)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0xE2 => {
                // LD (0xFF00+C), A
                let addr = 0xFF00 + self.registers.c as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }

            // 16-bit 加載指令
            0x01 => {
                // LD BC, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.c = lo;
                self.registers.b = hi;
            }
            0x11 => {
                // LD DE, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.e = lo;
                self.registers.d = hi;
            }
            0x21 => {
                // LD HL, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.l = lo;
                self.registers.h = hi;
            }
            0x31 => {
                // LD SP, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.sp = ((hi as u16) << 8) | (lo as u16);
            }

            // 遞增/遞減指令
            0x03 => {
                // INC BC
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_add(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = (result & 0xFF) as u8;
            }
            0x13 => {
                // INC DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = de.wrapping_add(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = (result & 0xFF) as u8;
            }
            0x23 => {
                // INC HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let result = hl.wrapping_add(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
            }
            0x33 => {
                // INC SP
                self.registers.sp = self.registers.sp.wrapping_add(1);
            }
            0x34 => {
                // INC (HL) - 遞增HL指向的記憶體值
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let value = self.mmu.read_byte(addr);
                let result = value.wrapping_add(1);
                self.mmu.write_byte(addr, result);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((value & 0x0F) == 0x0F);
            }
            0x0B => {
                // DEC BC
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let bc = bc.wrapping_sub(1);
                self.registers.b = (bc >> 8) as u8;
                self.registers.c = (bc & 0xFF) as u8;
            }
            0x1B => {
                // DEC DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let de = de.wrapping_sub(1);
                self.registers.d = (de >> 8) as u8;
                self.registers.e = (de & 0xFF) as u8;
            }
            0x2B => {
                // DEC HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = (hl & 0xFF) as u8;
            }
            0x3B => {
                // DEC SP
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }

            0x04 => {
                // INC B
                self.registers.b = self.registers.b.wrapping_add(1);
                self.registers.set_z_flag(self.registers.b == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.b & 0x0F) == 0);
            }
            0x0C => {
                // INC C
                self.registers.c = self.registers.c.wrapping_add(1);
                self.registers.set_z_flag(self.registers.c == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.c & 0x0F) == 0);
            }
            0x14 => {
                // INC D
                self.registers.d = self.registers.d.wrapping_add(1);
                self.registers.set_z_flag(self.registers.d == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.d & 0x0F) == 0);
            }
            0x1C => {
                // INC E
                self.registers.e = self.registers.e.wrapping_add(1);
                self.registers.set_z_flag(self.registers.e == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.e & 0x0F) == 0);
            }
            0x24 => {
                // INC H
                self.registers.h = self.registers.h.wrapping_add(1);
                self.registers.set_z_flag(self.registers.h == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.h & 0x0F) == 0);
            }
            0x2C => {
                // INC L
                self.registers.l = self.registers.l.wrapping_add(1);
                self.registers.set_z_flag(self.registers.l == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.l & 0x0F) == 0);
            }
            0x3C => {
                // INC A
                self.registers.a = self.registers.a.wrapping_add(1);
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((self.registers.a & 0x0F) == 0);
            }
            0x05 => {
                // DEC B
                self.registers.b = self.registers.b.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.b == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.b & 0x0F) == 0x0F);
            }
            0x0D => {
                // DEC C
                self.registers.c = self.registers.c.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.c == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.c & 0x0F) == 0x0F);
            }
            0x15 => {
                // DEC D
                self.registers.d = self.registers.d.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.d == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.d & 0x0F) == 0x0F);
            }
            0x1D => {
                // DEC E
                self.registers.e = self.registers.e.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.e == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.e & 0x0F) == 0x0F);
            }
            0x25 => {
                // DEC H
                self.registers.h = self.registers.h.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.h == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.h & 0x0F) == 0x0F);
            }
            0x2D => {
                // DEC L
                self.registers.l = self.registers.l.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.l == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.l & 0x0F) == 0x0F);
            }
            0x35 => {
                // DEC (HL) - 遞減HL指向的記憶體值
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let value = self.mmu.read_byte(addr);
                let result = value.wrapping_sub(1);
                self.mmu.write_byte(addr, result);

                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((value & 0x0F) == 0);
            }
            0x3D => {
                // DEC A
                self.registers.a = self.registers.a.wrapping_sub(1);
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(true);
                self.registers.set_h_flag((self.registers.a & 0x0F) == 0x0F);
            }

            // HL 相關特殊操作
            0x22 => {
                // LD (HL+),A
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
                let hl = addr.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = (hl & 0xFF) as u8;
            }
            0x2A => {
                // LD A,(HL+)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
                let hl = addr.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = (hl & 0xFF) as u8;
            }
            0x32 => {
                // LD (HL-),A
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
                let hl = addr.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = (hl & 0xFF) as u8;
            }
            0x3A => {
                // LD A,(HL-)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
                let hl = addr.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = (hl & 0xFF) as u8;
            }

            // 記憶體載入/儲存指令
            0x02 => {
                // LD (BC),A
                let addr = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x12 => {
                // LD (DE),A
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }

            // 8-bit LD r, r' 指令 (0x40~0x7F)
            0x40 => {
                self.registers.b = self.registers.b;
            } // LD B,B
            0x41 => {
                self.registers.b = self.registers.c;
            } // LD B,C
            0x42 => {
                self.registers.b = self.registers.d;
            } // LD B,D
            0x43 => {
                self.registers.b = self.registers.e;
            } // LD B,E
            0x44 => {
                self.registers.b = self.registers.h;
            } // LD B,H
            0x45 => {
                self.registers.b = self.registers.l;
            } // LD B,L
            0x46 => {
                // LD B,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.b = self.mmu.read_byte(addr);
            }
            0x47 => {
                self.registers.b = self.registers.a;
            } // LD B,A
            0x48 => {
                self.registers.c = self.registers.b;
            } // LD C,B
            0x49 => {
                self.registers.c = self.registers.c;
            } // LD C,C
            0x4A => {
                self.registers.c = self.registers.d;
            } // LD C,D
            0x4B => {
                self.registers.c = self.registers.e;
            } // LD C,E
            0x4C => {
                self.registers.c = self.registers.h;
            } // LD C,H
            0x4D => {
                self.registers.c = self.registers.l;
            } // LD C,L
            0x4E => {
                // LD C,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.c = self.mmu.read_byte(addr);
            }
            0x4F => {
                self.registers.c = self.registers.a;
            } // LD C,A
            0x50 => {
                self.registers.d = self.registers.b;
            } // LD D,B
            0x51 => {
                self.registers.d = self.registers.c;
            } // LD D,C
            0x52 => {
                self.registers.d = self.registers.d;
            } // LD D,D
            0x53 => {
                self.registers.d = self.registers.e;
            } // LD D,E
            0x54 => {
                self.registers.d = self.registers.h;
            } // LD D,H
            0x55 => {
                self.registers.d = self.registers.l;
            } // LD D,L
            0x56 => {
                // LD D,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.d = self.mmu.read_byte(addr);
            }
            0x57 => {
                self.registers.d = self.registers.a;
            } // LD D,A
            0x58 => {
                self.registers.e = self.registers.b;
            } // LD E,B
            0x59 => {
                self.registers.e = self.registers.c;
            } // LD E,C
            0x5A => {
                self.registers.e = self.registers.d;
            } // LD E,D
            0x5B => {
                self.registers.e = self.registers.e;
            } // LD E,E
            0x5C => {
                self.registers.e = self.registers.h;
            } // LD E,H
            0x5D => {
                self.registers.e = self.registers.l;
            } // LD E,L
            0x5E => {
                // LD E,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.e = self.mmu.read_byte(addr);
            }
            0x5F => {
                self.registers.e = self.registers.a;
            } // LD E,A
            0x60 => {
                self.registers.h = self.registers.b;
            } // LD H,B
            0x61 => {
                self.registers.h = self.registers.c;
            } // LD H,C
            0x62 => {
                self.registers.h = self.registers.d;
            } // LD H,D
            0x63 => {
                self.registers.h = self.registers.e;
            } // LD H,E
            0x64 => {
                self.registers.h = self.registers.h;
            } // LD H,H
            0x65 => {
                self.registers.h = self.registers.l;
            } // LD H,L
            0x66 => {
                // LD H,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.h = self.mmu.read_byte(addr);
            }
            0x67 => {
                self.registers.h = self.registers.a;
            } // LD H,A
            0x68 => {
                self.registers.l = self.registers.b;
            } // LD L,B
            0x69 => {
                self.registers.l = self.registers.c;
            } // LD L,C
            0x6A => {
                self.registers.l = self.registers.d;
            } // LD L,D
            0x6B => {
                self.registers.l = self.registers.e;
            } // LD L,E
            0x6C => {
                self.registers.l = self.registers.h;
            } // LD L,H
            0x6D => {
                self.registers.l = self.registers.l;
            } // LD L,L
            0x6E => {
                // LD L,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.l = self.mmu.read_byte(addr);
            }
            0x6F => {
                self.registers.l = self.registers.a;
            } // LD L,A
            0x70 => {
                // LD (HL),B
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.b);
            }
            0x71 => {
                // LD (HL),C
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.c);
            }
            0x72 => {
                // LD (HL),D
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.d);
            }
            0x73 => {
                // LD (HL),E
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.e);
            }
            0x74 => {
                // LD (HL),H
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.h);
            }
            0x75 => {
                // LD (HL),L
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.l);
            }
            0x77 => {
                // LD (HL),A
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x78 => {
                self.registers.a = self.registers.b;
            } // LD A,B
            0x79 => {
                self.registers.a = self.registers.c;
            } // LD A,C
            0x7A => {
                self.registers.a = self.registers.d;
            } // LD A,D
            0x7B => {
                self.registers.a = self.registers.e;
            } // LD A,E
            0x7C => {
                self.registers.a = self.registers.h;
            } // LD A,H
            0x7D => {
                self.registers.a = self.registers.l;
            } // LD A,L
            0x7E => {
                // LD A,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x7F => {
                self.registers.a = self.registers.a;
            } // LD A,A

            // 8-bit 算術與邏輯操作
            0x80 => {
                self.alu_add(self.registers.b);
            } // ADD A,B
            0x81 => {
                self.alu_add(self.registers.c);
            } // ADD A,C
            0x82 => {
                self.alu_add(self.registers.d);
            } // ADD A,D
            0x83 => {
                self.alu_add(self.registers.e);
            } // ADD A,E
            0x84 => {
                self.alu_add(self.registers.h);
            } // ADD A,H
            0x85 => {
                self.alu_add(self.registers.l);
            } // ADD A,L
            0x86 => {
                // ADD A,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_add(v);
            }
            0x87 => {
                self.alu_add(self.registers.a);
            } // ADD A,A
            0x88 => {
                self.alu_adc(self.registers.b);
            } // ADC A,B
            0x89 => {
                self.alu_adc(self.registers.c);
            } // ADC A,C
            0x8A => {
                self.alu_adc(self.registers.d);
            } // ADC A,D
            0x8B => {
                self.alu_adc(self.registers.e);
            } // ADC A,E
            0x8C => {
                self.alu_adc(self.registers.h);
            } // ADC A,H
            0x8D => {
                self.alu_adc(self.registers.l);
            } // ADC A,L
            0x8E => {
                // ADC A,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_adc(v);
            }
            0x8F => {
                self.alu_adc(self.registers.a);
            } // ADC A,A
            0x90 => {
                self.alu_sub(self.registers.b);
            } // SUB B
            0x91 => {
                self.alu_sub(self.registers.c);
            } // SUB C
            0x92 => {
                self.alu_sub(self.registers.d);
            } // SUB D
            0x93 => {
                self.alu_sub(self.registers.e);
            } // SUB E
            0x94 => {
                self.alu_sub(self.registers.h);
            } // SUB H
            0x95 => {
                self.alu_sub(self.registers.l);
            } // SUB L
            0x96 => {
                // SUB (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_sub(v);
            }
            0x97 => {
                self.alu_sub(self.registers.a);
            } // SUB A
            0x98 => {
                self.alu_sbc(self.registers.b);
            } // SBC A,B
            0x99 => {
                self.alu_sbc(self.registers.c);
            } // SBC A,C
            0x9A => {
                self.alu_sbc(self.registers.d);
            } // SBC A,D
            0x9B => {
                self.alu_sbc(self.registers.e);
            } // SBC A,E
            0x9C => {
                self.alu_sbc(self.registers.h);
            } // SBC A,H
            0x9D => {
                self.alu_sbc(self.registers.l);
            } // SBC A,L
            0x9E => {
                // SBC A,(HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_sbc(v);
            }
            0x9F => {
                self.alu_sbc(self.registers.a);
            } // SBC A,A
            0xA0 => {
                self.alu_and(self.registers.b);
            } // AND B
            0xA1 => {
                self.alu_and(self.registers.c);
            } // AND C
            0xA2 => {
                self.alu_and(self.registers.d);
            } // AND D
            0xA3 => {
                self.alu_and(self.registers.e);
            } // AND E
            0xA4 => {
                self.alu_and(self.registers.h);
            } // AND H
            0xA5 => {
                self.alu_and(self.registers.l);
            } // AND L
            0xA6 => {
                // AND (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_and(v);
            }
            0xA7 => {
                self.alu_and(self.registers.a);
            } // AND A
            0xA8 => {
                self.alu_xor(self.registers.b);
            } // XOR B
            0xA9 => {
                self.alu_xor(self.registers.c);
            } // XOR C
            0xAA => {
                self.alu_xor(self.registers.d);
            } // XOR D
            0xAB => {
                self.alu_xor(self.registers.e);
            } // XOR E
            0xAC => {
                self.alu_xor(self.registers.h);
            } // XOR H
            0xAD => {
                self.alu_xor(self.registers.l);
            } // XOR L
            0xAE => {
                // XOR (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_xor(v);
            }
            0xAF => {
                self.alu_xor(self.registers.a);
            } // XOR A
            0xB0 => {
                self.alu_or(self.registers.b);
            } // OR B
            0xB1 => {
                self.alu_or(self.registers.c);
            } // OR C
            0xB2 => {
                self.alu_or(self.registers.d);
            } // OR D
            0xB3 => {
                self.alu_or(self.registers.e);
            } // OR E
            0xB4 => {
                self.alu_or(self.registers.h);
            } // OR H
            0xB5 => {
                self.alu_or(self.registers.l);
            } // OR L
            0xB6 => {
                // OR (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_or(v);
            }
            0xB7 => {
                self.alu_or(self.registers.a);
            } // OR A
            0xB8 => {
                self.alu_cp(self.registers.b);
            } // CP B
            0xB9 => {
                self.alu_cp(self.registers.c);
            } // CP C
            0xBA => {
                self.alu_cp(self.registers.d);
            } // CP D
            0xBB => {
                self.alu_cp(self.registers.e);
            } // CP E
            0xBC => {
                self.alu_cp(self.registers.h);
            } // CP H
            0xBD => {
                self.alu_cp(self.registers.l);
            } // CP L
            0xBE => {
                // CP (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_cp(v);
            }
            0xBF => {
                self.alu_cp(self.registers.a);
            } // CP A
            // 立即數算術指令
            0xFE => {
                // CP n (Compare A with immediate value n)
                let n = self.fetch();
                self.alu_cp(n);
            }
            // 中斷控制指令
            0xF3 => {
                // DI (Disable Interrupts)
                // 在真實的Game Boy中，這會禁用中斷
                // 目前簡化實現，不做任何操作
            }
            0xFB => {
                // EI (Enable Interrupts)
                // 在真實的Game Boy中，這會啟用中斷
                // 目前簡化實現，不做任何操作
            }

            // 記憶體操作指令
            0x36 => {
                // LD (HL), n (載入立即數到HL指向的記憶體)
                let n = self.fetch();
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, n);
            }
            0xEA => {
                // LD (nn), A (載入A到絕對地址nn)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, self.registers.a);
            }

            // 子程序調用指令
            0xCD => {
                // CALL nn (調用子程序)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;

                // 將當前PC推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                // 跳轉到目標地址
                self.registers.pc = addr;
            }

            // 條件跳轉指令
            0xCA => {
                // JP Z, nn (如果Z標誌設置則跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let z_flag = (self.registers.f & 0x80) != 0;
                if z_flag {
                    self.registers.pc = addr;
                }
            }

            // RST指令（重啟到固定地址）
            0xCF => {
                // RST 08H (重啟到地址0x08)
                // 將當前PC推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                // 跳轉到0x08
                self.registers.pc = 0x08;
            }
            0xFF => {
                // RST 38H (重啟到地址0x38)
                // 將當前PC推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                // 跳轉到0x38
                self.registers.pc = 0x38;
            }

            // 算術指令
            0x27 => {
                // DAA (十進制調整累加器)
                let mut a = self.registers.a;
                let mut adjust = 0;

                if (self.registers.f & 0x20) != 0
                    || (!((self.registers.f & 0x40) != 0) && (a & 0x0F) > 9)
                {
                    adjust |= 0x06;
                }

                if (self.registers.f & 0x10) != 0 || (!((self.registers.f & 0x40) != 0) && a > 0x99)
                {
                    adjust |= 0x60;
                    self.registers.set_c_flag(true);
                }

                if (self.registers.f & 0x40) != 0 {
                    a = a.wrapping_sub(adjust);
                } else {
                    a = a.wrapping_add(adjust);
                }

                self.registers.set_z_flag(a == 0);
                self.registers.set_h_flag(false);
                self.registers.a = a;
            }
            0x29 => {
                // ADD HL, HL (HL加上自身)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let result = hl.wrapping_add(hl);

                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag((hl & 0x0FFF) + (hl & 0x0FFF) > 0x0FFF);
                self.registers.set_c_flag(result < hl);

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
            }

            // 特殊載入指令
            0xF8 => {
                // LD HL, SP+n (載入SP+偏移到HL)
                let offset = self.fetch() as i8;
                let sp = self.registers.sp;
                let result = (sp as i32 + offset as i32) as u16;

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag((sp & 0x0F) + ((offset as u16) & 0x0F) > 0x0F);
                self.registers
                    .set_c_flag((sp & 0xFF) + ((offset as u16) & 0xFF) > 0xFF);

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
            }

            // 邏輯指令
            0xE6 => {
                // AND n (邏輯AND立即數)
                let n = self.fetch();
                self.alu_and(n);
            }
            0x0F => {
                // RRCA (右旋轉累加器)
                let a = self.registers.a;
                let result = (a >> 1) | ((a & 0x01) << 7);

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag((a & 0x01) != 0);

                self.registers.a = result;
            }

            // 添加缺失的指令
            0x07 => {
                // RLCA (左旋轉累加器)
                let a = self.registers.a;
                let result = (a << 1) | (a >> 7);

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag((a & 0x80) != 0);

                self.registers.a = result;
            }
            0x08 => {
                // LD (nn), SP (載入SP到絕對地址)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, (self.registers.sp & 0xFF) as u8);
                self.mmu
                    .write_byte(addr + 1, (self.registers.sp >> 8) as u8);
            }
            0x09 => {
                // ADD HL, BC (將BC加到HL)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = hl.wrapping_add(bc);

                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag((hl & 0x0FFF) + (bc & 0x0FFF) > 0x0FFF);
                self.registers.set_c_flag(result < hl);

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
            }
            0x0A => {
                // LD A, (BC)
                let addr = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x17 => {
                // RLA (左旋轉累加器通過進位)
                let a = self.registers.a;
                let c = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
                let result = (a << 1) | c;

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag((a & 0x80) != 0);

                self.registers.a = result;
            }
            0x19 => {
                // ADD HL, DE
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = hl.wrapping_add(de);

                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag((hl & 0x0FFF) + (de & 0x0FFF) > 0x0FFF);
                self.registers.set_c_flag(result < hl);

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
            }
            0x1A => {
                // LD A, (DE)
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x1F => {
                // RRA (右旋轉累加器通過進位)
                let a = self.registers.a;
                let c = if (self.registers.f & 0x10) != 0 {
                    0x80
                } else {
                    0
                };
                let result = (a >> 1) | c;

                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag((a & 0x01) != 0);

                self.registers.a = result;
            }
            0x2F => {
                // CPL (補數累加器)
                self.registers.a = !self.registers.a;
                self.registers.set_n_flag(true);
                self.registers.set_h_flag(true);
            }
            0x37 => {
                // SCF (設置進位標誌)
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(true);
            }
            0x3F => {
                // CCF (補數進位標誌)
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                let c = (self.registers.f & 0x10) != 0;
                self.registers.set_c_flag(!c);
            }

            // 條件返回指令
            0xC0 => {
                // RET NZ (如果Z標誌未設置則返回)
                let z_flag = (self.registers.f & 0x80) != 0;
                if !z_flag {
                    let sp = self.registers.sp;
                    let lo = self.mmu.read_byte(sp) as u16;
                    let hi = self.mmu.read_byte(sp + 1) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(2);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0xC8 => {
                // RET Z (如果Z標誌設置則返回)
                let z_flag = (self.registers.f & 0x80) != 0;
                if z_flag {
                    let sp = self.registers.sp;
                    let lo = self.mmu.read_byte(sp) as u16;
                    let hi = self.mmu.read_byte(sp + 1) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(2);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0xD0 => {
                // RET NC (如果C標誌未設置則返回)
                let c_flag = (self.registers.f & 0x10) != 0;
                if !c_flag {
                    let sp = self.registers.sp;
                    let lo = self.mmu.read_byte(sp) as u16;
                    let hi = self.mmu.read_byte(sp + 1) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(2);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0xD8 => {
                // RET C (如果C標誌設置則返回)
                let c_flag = (self.registers.f & 0x10) != 0;
                if c_flag {
                    let sp = self.registers.sp;
                    let lo = self.mmu.read_byte(sp) as u16;
                    let hi = self.mmu.read_byte(sp + 1) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(2);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0xD9 => {
                // RETI (中斷返回)
                let sp = self.registers.sp;
                let lo = self.mmu.read_byte(sp) as u16;
                let hi = self.mmu.read_byte(sp + 1) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.pc = (hi << 8) | lo;
                self.ime = true; // 啟用中斷
            }

            // 條件跳轉指令
            0xC2 => {
                // JP NZ, nn (如果Z標誌未設置則跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let z_flag = (self.registers.f & 0x80) != 0;
                if !z_flag {
                    self.registers.pc = addr;
                }
            }
            0xD2 => {
                // JP NC, nn (如果C標誌未設置則跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let c_flag = (self.registers.f & 0x10) != 0;
                if !c_flag {
                    self.registers.pc = addr;
                }
            }

            // 條件調用指令
            0xC4 => {
                // CALL NZ, nn (如果Z標誌未設置則調用)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let z_flag = (self.registers.f & 0x80) != 0;
                if !z_flag {
                    // 將當前PC推入堆疊
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                    // 跳轉到目標地址
                    self.registers.pc = addr;
                }
            }
            0xCC => {
                // CALL Z, nn (如果Z標誌設置則調用)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let z_flag = (self.registers.f & 0x80) != 0;
                if z_flag {
                    // 將當前PC推入堆疊
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                    // 跳轉到目標地址
                    self.registers.pc = addr;
                }
            }
            0xD4 => {
                // CALL NC, nn (如果C標誌未設置則調用)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let c_flag = (self.registers.f & 0x10) != 0;
                if !c_flag {
                    // 將當前PC推入堆疊
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                    // 跳轉到目標地址
                    self.registers.pc = addr;
                }
            }
            0xDC => {
                // CALL C, nn (如果C標誌設置則調用)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let c_flag = (self.registers.f & 0x10) != 0;
                if c_flag {
                    // 將當前PC推入堆疊
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);

                    // 跳轉到目標地址
                    self.registers.pc = addr;
                }
            }

            // 立即數運算指令
            0xC6 => {
                // ADD A, n
                let n = self.fetch();
                self.alu_add(n);
            }
            0xCE => {
                // ADC A, n
                let n = self.fetch();
                self.alu_adc(n);
            }
            0xD6 => {
                // SUB n
                let n = self.fetch();
                self.alu_sub(n);
            }
            0xDE => {
                // SBC A, n
                let n = self.fetch();
                self.alu_sbc(n);
            }
            0xEE => {
                // XOR n
                let n = self.fetch();
                self.alu_xor(n);
            }
            0xF6 => {
                // OR n
                let n = self.fetch();
                self.alu_or(n);
            }

            // 堆疊操作指令
            0xC1 => {
                // POP BC
                let lo = self.mmu.read_byte(self.registers.sp);
                let hi = self.mmu.read_byte(self.registers.sp + 1);
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.c = lo;
                self.registers.b = hi;
            }
            0xC5 => {
                // PUSH BC
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.b);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.c);
            }
            0xD1 => {
                // POP DE
                let lo = self.mmu.read_byte(self.registers.sp);
                let hi = self.mmu.read_byte(self.registers.sp + 1);
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.e = lo;
                self.registers.d = hi;
            }
            0xD5 => {
                // PUSH DE
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.d);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.e);
            }
            0xE1 => {
                // POP HL
                let lo = self.mmu.read_byte(self.registers.sp);
                let hi = self.mmu.read_byte(self.registers.sp + 1);
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.l = lo;
                self.registers.h = hi;
            }
            0xE5 => {
                // PUSH HL
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.h);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.l);
            }
            0xF1 => {
                // POP AF
                let lo = self.mmu.read_byte(self.registers.sp);
                let hi = self.mmu.read_byte(self.registers.sp + 1);
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.f = lo & 0xF0; // 下4位總是0
                self.registers.a = hi;
            }
            0xF5 => {
                // PUSH AF
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.a);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.f);
            }

            // RST指令（重啟到固定地址）
            0xC7 => {
                // RST 00H (重啟到地址0x00)
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x00;
            }
            0xD7 => {
                // RST 10H (重啟到地址0x10)
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x10;
            }
            0xDF => {
                // RST 18H (重啟到地址0x18)
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x18;
            }
            0xE7 => {
                // RST 20H (重啟到地址0x20)
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x20;
            }
            0xEF => {
                // RST 28H (重啟到地址0x28)
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x28;
            }
            0xF7 => {
                // RST 30H (重啟到地址0x30)
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x30;
            }

            // 其他指令
            0xE4 => {
                // LD (n), A (載入A到高記憶體, 等同於LDH)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xE9 => {
                // JP (HL) (跳轉到HL指向的地址)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.pc = addr;
            }
            0xF9 => {
                // LD SP, HL (載入HL到SP)
                self.registers.sp = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
            }
            0xFC => {
                // CALL C, nn (如果C標誌設置則調用 - 應該已有實現，重複指令)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let c_flag = (self.registers.f & 0x10) != 0;
                if c_flag {
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                    self.registers.pc = addr;
                }
            }
            // 0xCB 前綴指令 - 執行位元操作指令
            0xCB => {
                let cb_opcode = self.fetch();
                match cb_opcode {
                    // RLC r (Rotate Left Circular)
                    0x00 => {
                        self.registers.b = self.rlc(self.registers.b);
                    }
                    0x01 => {
                        self.registers.c = self.rlc(self.registers.c);
                    }
                    0x02 => {
                        self.registers.d = self.rlc(self.registers.d);
                    }
                    0x03 => {
                        self.registers.e = self.rlc(self.registers.e);
                    }
                    0x04 => {
                        self.registers.h = self.rlc(self.registers.h);
                    }
                    0x05 => {
                        self.registers.l = self.rlc(self.registers.l);
                    }
                    0x06 => {
                        // RLC (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rlc(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x07 => {
                        self.registers.a = self.rlc(self.registers.a);
                    }

                    // RRC r (Rotate Right Circular)
                    0x08 => {
                        self.registers.b = self.rrc(self.registers.b);
                    }
                    0x09 => {
                        self.registers.c = self.rrc(self.registers.c);
                    }
                    0x0A => {
                        self.registers.d = self.rrc(self.registers.d);
                    }
                    0x0B => {
                        self.registers.e = self.rrc(self.registers.e);
                    }
                    0x0C => {
                        self.registers.h = self.rrc(self.registers.h);
                    }
                    0x0D => {
                        self.registers.l = self.rrc(self.registers.l);
                    }
                    0x0E => {
                        // RRC (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rrc(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x0F => {
                        self.registers.a = self.rrc(self.registers.a);
                    }

                    // RL r (Rotate Left through Carry)
                    0x10 => {
                        self.registers.b = self.rl(self.registers.b);
                    }
                    0x11 => {
                        self.registers.c = self.rl(self.registers.c);
                    }
                    0x12 => {
                        self.registers.d = self.rl(self.registers.d);
                    }
                    0x13 => {
                        self.registers.e = self.rl(self.registers.e);
                    }
                    0x14 => {
                        self.registers.h = self.rl(self.registers.h);
                    }
                    0x15 => {
                        self.registers.l = self.rl(self.registers.l);
                    }
                    0x16 => {
                        // RL (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rl(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x17 => {
                        self.registers.a = self.rl(self.registers.a);
                    }

                    // RR r (Rotate Right through Carry)
                    0x18 => {
                        self.registers.b = self.rr(self.registers.b);
                    }
                    0x19 => {
                        self.registers.c = self.rr(self.registers.c);
                    }
                    0x1A => {
                        self.registers.d = self.rr(self.registers.d);
                    }
                    0x1B => {
                        self.registers.e = self.rr(self.registers.e);
                    }
                    0x1C => {
                        self.registers.h = self.rr(self.registers.h);
                    }
                    0x1D => {
                        self.registers.l = self.rr(self.registers.l);
                    }
                    0x1E => {
                        // RR (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rr(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x1F => {
                        self.registers.a = self.rr(self.registers.a);
                    }

                    // SLA r (Shift Left Arithmetic)
                    0x20 => {
                        self.registers.b = self.sla(self.registers.b);
                    }
                    0x21 => {
                        self.registers.c = self.sla(self.registers.c);
                    }
                    0x22 => {
                        self.registers.d = self.sla(self.registers.d);
                    }
                    0x23 => {
                        self.registers.e = self.sla(self.registers.e);
                    }
                    0x24 => {
                        self.registers.h = self.sla(self.registers.h);
                    }
                    0x25 => {
                        self.registers.l = self.sla(self.registers.l);
                    }
                    0x26 => {
                        // SLA (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.sla(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x27 => {
                        self.registers.a = self.sla(self.registers.a);
                    }

                    // SRA r (Shift Right Arithmetic)
                    0x28 => {
                        self.registers.b = self.sra(self.registers.b);
                    }
                    0x29 => {
                        self.registers.c = self.sra(self.registers.c);
                    }
                    0x2A => {
                        self.registers.d = self.sra(self.registers.d);
                    }
                    0x2B => {
                        self.registers.e = self.sra(self.registers.e);
                    }
                    0x2C => {
                        self.registers.h = self.sra(self.registers.h);
                    }
                    0x2D => {
                        self.registers.l = self.sra(self.registers.l);
                    }
                    0x2E => {
                        // SRA (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.sra(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x2F => {
                        self.registers.a = self.sra(self.registers.a);
                    }

                    // SWAP r (Swap nibbles)
                    0x30 => {
                        self.registers.b = self.swap(self.registers.b);
                    }
                    0x31 => {
                        self.registers.c = self.swap(self.registers.c);
                    }
                    0x32 => {
                        self.registers.d = self.swap(self.registers.d);
                    }
                    0x33 => {
                        self.registers.e = self.swap(self.registers.e);
                    }
                    0x34 => {
                        self.registers.h = self.swap(self.registers.h);
                    }
                    0x35 => {
                        self.registers.l = self.swap(self.registers.l);
                    }
                    0x36 => {
                        // SWAP (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.swap(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x37 => {
                        self.registers.a = self.swap(self.registers.a);
                    }

                    // SRL r (Shift Right Logical)
                    0x38 => {
                        self.registers.b = self.srl(self.registers.b);
                    }
                    0x39 => {
                        self.registers.c = self.srl(self.registers.c);
                    }
                    0x3A => {
                        self.registers.d = self.srl(self.registers.d);
                    }
                    0x3B => {
                        self.registers.e = self.srl(self.registers.e);
                    }
                    0x3C => {
                        self.registers.h = self.srl(self.registers.h);
                    }
                    0x3D => {
                        self.registers.l = self.srl(self.registers.l);
                    }
                    0x3E => {
                        // SRL (HL)
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.srl(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x3F => {
                        self.registers.a = self.srl(self.registers.a);
                    }

                    // BIT b, r (Test bit b in register r)
                    0x40..=0x7F => {
                        let bit = (cb_opcode - 0x40) / 8;
                        let reg = (cb_opcode - 0x40) % 8;
                        let value = match reg {
                            0 => self.registers.b,
                            1 => self.registers.c,
                            2 => self.registers.d,
                            3 => self.registers.e,
                            4 => self.registers.h,
                            5 => self.registers.l,
                            6 => {
                                let addr =
                                    ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                                self.mmu.read_byte(addr)
                            }
                            7 => self.registers.a,
                            _ => 0, // 不應該出現的情況
                        };
                        let z = (value & (1 << bit)) == 0;
                        self.registers.set_z_flag(z);
                        self.registers.set_n_flag(false);
                        self.registers.set_h_flag(true);
                    }

                    // RES b, r (Reset bit b in register r)
                    0x80..=0xBF => {
                        let bit = (cb_opcode - 0x80) / 8;
                        let reg = (cb_opcode - 0x80) % 8;
                        match reg {
                            0 => self.registers.b &= !(1 << bit),
                            1 => self.registers.c &= !(1 << bit),
                            2 => self.registers.d &= !(1 << bit),
                            3 => self.registers.e &= !(1 << bit),
                            4 => self.registers.h &= !(1 << bit),
                            5 => self.registers.l &= !(1 << bit),
                            6 => {
                                let addr =
                                    ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                                let v = self.mmu.read_byte(addr) & !(1 << bit);
                                self.mmu.write_byte(addr, v);
                            }
                            7 => self.registers.a &= !(1 << bit),
                            _ => {} // 不應該出現的情況
                        }
                    }

                    // SET b, r (Set bit b in register r)
                    0xC0..=0xFF => {
                        let bit = (cb_opcode - 0xC0) / 8;
                        let reg = (cb_opcode - 0xC0) % 8;
                        match reg {
                            0 => self.registers.b |= 1 << bit,
                            1 => self.registers.c |= 1 << bit,
                            2 => self.registers.d |= 1 << bit,
                            3 => self.registers.e |= 1 << bit,
                            4 => self.registers.h |= 1 << bit,
                            5 => self.registers.l |= 1 << bit,
                            6 => {
                                let addr =
                                    ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                                let v = self.mmu.read_byte(addr) | (1 << bit);
                                self.mmu.write_byte(addr, v);
                            }
                            7 => self.registers.a |= 1 << bit,
                            _ => {} // 不應該出現的情況
                        }
                    }
                }
            }

            // 未實現的指令會輸出提示
            _ => {
                println!("未處理的指令: 0x{:02X}", opcode);
            }
        }
    }

    fn handle_interrupts(&mut self) {
        if !self.ime {
            return; // 中斷被禁用
        }

        let if_reg = self.mmu.read_byte(0xFF0F); // 中斷標誌寄存器
        let ie_reg = self.mmu.read_byte(0xFFFF); // 中斷啟用寄存器

        let pending_interrupts = if_reg & ie_reg;

        if pending_interrupts != 0 {
            // 有待處理的中斷
            self.ime = false; // 禁用中斷

            // 檢查手柄中斷 (bit 4)
            if (pending_interrupts & 0x10) != 0 {
                println!("🚨 處理手柄中斷!");
                // 清除手柄中斷標誌
                let new_if = if_reg & !0x10;
                self.mmu.write_byte(0xFF0F, new_if);

                // 跳轉到手柄中斷處理程序 (0x0060)
                self.push_word(self.registers.pc);
                self.registers.pc = 0x0060;
                return;
            }

            // 檢查VBlank中斷 (bit 0)
            if (pending_interrupts & 0x01) != 0 {
                // 清除VBlank中斷標誌
                let new_if = if_reg & !0x01;
                self.mmu.write_byte(0xFF0F, new_if);

                // 跳轉到VBlank中斷處理程序 (0x0040)
                self.push_word(self.registers.pc);
                self.registers.pc = 0x0040;
                return;
            }

            // 檢查其他中斷 (LCDC, Timer, Serial)
            if (pending_interrupts & 0x02) != 0 {
                // LCDC 中斷
                let new_if = if_reg & !0x02;
                self.mmu.write_byte(0xFF0F, new_if);
                self.push_word(self.registers.pc);
                self.registers.pc = 0x0048;
                return;
            }

            if (pending_interrupts & 0x04) != 0 {
                // Timer 中斷
                let new_if = if_reg & !0x04;
                self.mmu.write_byte(0xFF0F, new_if);
                self.push_word(self.registers.pc);
                self.registers.pc = 0x0050;
                return;
            }

            if (pending_interrupts & 0x08) != 0 {
                // Serial 中斷
                let new_if = if_reg & !0x08;
                self.mmu.write_byte(0xFF0F, new_if);
                self.push_word(self.registers.pc);
                self.registers.pc = 0x0058;
                return;
            }
        }
    }

    fn push_word(&mut self, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.mmu.write_byte(self.registers.sp, (value >> 8) as u8);
        self.registers.sp = self.registers.sp.wrapping_sub(1);
        self.mmu.write_byte(self.registers.sp, value as u8);
    }
}
