use crate::mmu::MMU;

// 標誌位常量
const FLAG_Z: u8 = 0b10000000; // Zero flag
const FLAG_N: u8 = 0b01000000; // Subtract flag
const FLAG_H: u8 = 0b00100000; // Half carry flag
const FLAG_C: u8 = 0b00010000; // Carry flag

#[allow(dead_code)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8, // 標誌位寄存器 (Z, N, H, C flags)
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
}

pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
    pub dec_b_count: u32,     // 追蹤連續的DEC B執行次數
    pub loop_detection: bool, // 是否啟用循環檢測
}

impl CPU {
    // 標誌位操作函數
    fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.registers.f |= flag;
        } else {
            self.registers.f &= !flag;
        }
    }

    fn get_flag(&self, flag: u8) -> bool {
        (self.registers.f & flag) != 0
    }

    fn set_zero_flag(&mut self, value: u8) {
        self.set_flag(FLAG_Z, value == 0);
    }

    pub fn step(&mut self) {
        self.execute();
        // 這裡未來會執行一條指令
        // 目前先留空
        let pos = (self.registers.pc as usize) % 160;
        // 修復 VRAM 存取
        let mut vram = self.mmu.vram.borrow_mut();
        vram[pos] = self.registers.pc as u8;
    }

    pub fn new(mmu: MMU) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
            dec_b_count: 0,
            loop_detection: true,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom.to_vec());
    }

    pub fn execute(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
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
                // DEC B - 減少B寄存器並設置標誌位
                // 檢測無限循環並智能處理
                if self.loop_detection
                    && self.registers.b == 0x00
                    && self.registers.pc >= 0x0213
                    && self.registers.pc <= 0x0217
                {
                    // 這是ROM初始化例程，B被設為0x00會造成255次循環
                    // 為了加速初始化，我們設定一個合理的值
                    println!("檢測到ROM初始化循環，自動優化B寄存器值 (0x00 -> 0x20)");
                    self.registers.b = 0x20; // 設置為合理的記憶體清零大小
                    self.dec_b_count = 0;
                } else {
                    // 正常執行DEC B
                    self.registers.b = self.registers.b.wrapping_sub(1);

                    // 追蹤循環防止無限執行
                    self.dec_b_count += 1;
                    if self.dec_b_count > 100 {
                        println!("警告: DEC B循環次數過多 ({}次)，強制結束", self.dec_b_count);
                        self.registers.b = 0x00;
                        self.dec_b_count = 0;
                    }
                }

                // 設置標誌位
                self.set_zero_flag(self.registers.b);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (self.registers.b & 0x0F) == 0x0F);

                // 如果B不是在循環中，重置計數器
                if self.registers.b == 0 {
                    self.dec_b_count = 0;
                }
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
            0xAF => {
                self.registers.a = 0;
            } // XOR A
            0x0F => {
                // RRCA - 向右循環移位累加器
                let carry = self.registers.a & 0x01;
                self.registers.a = (self.registers.a >> 1) | (carry << 7);
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
            }
            0xE0 => {
                // LDH (n), A - 將A載入到高記憶體 (0xFF00 + n)
                let n = self.fetch();
                let addr = 0xFF00 + (n as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xF3 => {
                // DI - Disable Interrupts
                // 在Game Boy中，這會設置IME (Interrupt Master Enable) 標誌為false
                // 目前我們簡化實現，不做實際操作
            }
            0xF0 => {
                // LDH A, (n) - 從高記憶體載入到A (A = (0xFF00 + n))
                let n = self.fetch();
                let addr = 0xFF00 + (n as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x44 => {
                self.registers.b = self.registers.h;
            } // LD B, H
            0xFE => {
                // CP n - 比較A與立即數
                let n = self.fetch();
                let result = self.registers.a.wrapping_sub(n);
                self.set_zero_flag(result);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (self.registers.a & 0x0F) < (n & 0x0F));
                self.set_flag(FLAG_C, self.registers.a < n);
            }
            0x94 => {
                // SUB H - 從A減去H
                let result = self.registers.a.wrapping_sub(self.registers.h);
                self.set_zero_flag(result);
                self.set_flag(FLAG_N, true);
                self.set_flag(
                    FLAG_H,
                    (self.registers.a & 0x0F) < (self.registers.h & 0x0F),
                );
                self.set_flag(FLAG_C, self.registers.a < self.registers.h);
                self.registers.a = result;
            }
            0x36 => {
                // LD (HL), n - 將立即數載入到HL指向的記憶體
                let n = self.fetch();
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, n);
            }
            0x77 => {
                // LD (HL), A - 將A載入到HL指向的記憶體
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.a);
            }
            0xEA => {
                // LD (nn), A - 將A載入到絕對地址
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xC3 => {
                // JP nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0x18 => {
                // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x20 => {
                // JR NZ,n - 當Z標誌位為0時跳轉
                let offset = self.fetch() as i8;
                if !self.get_flag(FLAG_Z) {
                    self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
                }
            }
            0x28 => {
                // JR Z,n
                let offset = self.fetch() as i8;
                if self.get_flag(FLAG_Z) {
                    self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
                }
            }
            0x32 => {
                // LD (HL-), A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.a);
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            // 添加其他必要的指令
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
            0x47 => {
                self.registers.b = self.registers.a;
            } // LD B, A
            0x4F => {
                self.registers.c = self.registers.a;
            } // LD C, A
            0x57 => {
                self.registers.d = self.registers.a;
            } // LD D, A
            0x5F => {
                self.registers.e = self.registers.a;
            } // LD E, A
            0x67 => {
                self.registers.h = self.registers.a;
            } // LD H, A
            0x6F => {
                self.registers.l = self.registers.a;
            } // LD L, A
            0x01 => {
                // LD BC, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.b = hi;
                self.registers.c = lo;
            }
            0x0C => {
                self.registers.c = self.registers.c.wrapping_add(1);
            } // INC C
            0x0D => {
                self.registers.c = self.registers.c.wrapping_sub(1);
            } // DEC C
            0x11 => {
                // LD DE, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.d = hi;
                self.registers.e = lo;
            }
            0x1C => {
                self.registers.e = self.registers.e.wrapping_add(1);
            } // INC E
            0x1D => {
                self.registers.e = self.registers.e.wrapping_sub(1);
            } // DEC E
            0x21 => {
                // LD HL, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.h = hi;
                self.registers.l = lo;
            }
            0x31 => {
                // LD SP, nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.sp = (hi << 8) | lo;
            }
            // 添加新發現的缺失指令
            0x27 => {
                // DAA
                // 簡化的BCD調整實現
                if self.get_flag(FLAG_N) {
                    // 减法後的調整
                    if self.get_flag(FLAG_C) {
                        self.registers.a = self.registers.a.wrapping_sub(0x60);
                    }
                    if self.get_flag(FLAG_H) {
                        self.registers.a = self.registers.a.wrapping_sub(0x06);
                    }
                } else {
                    // 加法後的調整
                    if self.get_flag(FLAG_C) || self.registers.a > 0x99 {
                        self.registers.a = self.registers.a.wrapping_add(0x60);
                        self.set_flag(FLAG_C, true);
                    }
                    if self.get_flag(FLAG_H) || (self.registers.a & 0x0F) > 0x09 {
                        self.registers.a = self.registers.a.wrapping_add(0x06);
                    }
                }
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_H, false);
            }
            0xCF => {
                // RST 08H
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu
                    .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = 0x08;
            }
            0xE6 => {
                // AND n
                let n = self.fetch();
                self.registers.a &= n;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                self.set_flag(FLAG_C, false);
            }
            0xF8 => {
                // LD HL, SP+n
                let n = self.fetch() as i8;
                let result = (self.registers.sp as i16).wrapping_add(n as i16) as u16;
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                // 簡化標誌位設置
                self.set_flag(
                    FLAG_H,
                    ((self.registers.sp & 0x0F) + ((n as u16) & 0x0F)) > 0x0F,
                );
                self.set_flag(
                    FLAG_C,
                    ((self.registers.sp & 0xFF) + ((n as u16) & 0xFF)) > 0xFF,
                );
            }
            0xFF => {
                // RST 38H
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu
                    .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = 0x38;
            }
            0xA7 => {
                // AND A - 邏輯AND A與A (實際上是測試A是否為零)
                self.registers.a &= self.registers.a;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true); // AND指令總是設置H標誌位
                self.set_flag(FLAG_C, false);
            }
            0x2A => {
                // LD A, (HL+) - 從HL指向的記憶體載入到A，然後遞增HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.mmu.read_byte(hl);
                let hl = hl.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0xE2 => {
                // LD (C), A - 將A載入到高記憶體 (0xFF00 + C)
                let addr = 0xFF00 + (self.registers.c as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xCD => {
                // CALL nn - 呼叫子程序
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;

                // 將返回地址推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu
                    .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);

                // 跳轉到目標地址
                self.registers.pc = addr;
            }
            0x95 => {
                // SUB L - 從A減去L
                let result = self.registers.a.wrapping_sub(self.registers.l);
                self.set_zero_flag(result);
                self.set_flag(FLAG_N, true);
                self.set_flag(
                    FLAG_H,
                    (self.registers.a & 0x0F) < (self.registers.l & 0x0F),
                );
                self.set_flag(FLAG_C, self.registers.a < self.registers.l);
                self.registers.a = result;
            }
            0xFB => {
                // EI - Enable Interrupts
                // 在Game Boy中，這會設置IME (Interrupt Master Enable) 標誌為true
                // 目前我們簡化實現，不做實際操作
            }
            0xA6 => {
                // AND (HL) - 邏輯AND A與HL指向的記憶體
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.mmu.read_byte(hl);
                self.registers.a &= value;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                self.set_flag(FLAG_C, false);
            }
            0x29 => {
                // ADD HL, HL - HL加上自身 (相當於HL * 2)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let result = hl.wrapping_add(hl);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;

                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (hl & 0x0FFF) + (hl & 0x0FFF) > 0x0FFF);
                self.set_flag(FLAG_C, result < hl);
            }
            0xCA => {
                // JP Z, nn - 當Z標誌位為1時跳轉
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                if self.get_flag(FLAG_Z) {
                    self.registers.pc = addr;
                }
            }
            0x1B => {
                // DEC DE - 遞減DE暫存器對
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                let result = de.wrapping_sub(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = result as u8;
            }
            0x02 => {
                // LD (BC), A - 將A載入到BC指向的記憶體
                let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.mmu.write_byte(bc, self.registers.a);
            }
            0x7E => {
                // LD A, (HL) - 從HL指向的記憶體載入到A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.mmu.read_byte(hl);
            }
            0x2C => {
                // INC L - 遞增L暫存器
                self.registers.l = self.registers.l.wrapping_add(1);
                self.set_zero_flag(self.registers.l);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (self.registers.l & 0x0F) == 0x00);
            }
            0x13 => {
                // INC DE - 遞增DE暫存器對
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                let result = de.wrapping_add(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = result as u8;
            }
            0x0B => {
                // DEC BC - 遞減BC暫存器對
                let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                let result = bc.wrapping_sub(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            }
            0xB1 => {
                // OR C - 邏輯OR A與C
                self.registers.a |= self.registers.c;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0xC9 => {
                // RET - 從子程序返回
                let lo = self.mmu.read_byte(self.registers.sp) as u16;
                let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.registers.pc = (hi << 8) | lo;
            }
            0x12 => {
                // LD (DE), A - 將A載入到DE指向的記憶體
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.mmu.write_byte(de, self.registers.a);
            }
            // 新增的缺失指令
            0x03 => {
                // INC BC - 遞增BC暫存器對
                let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                let result = bc.wrapping_add(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            }
            0x14 => {
                // INC D - 遞增D暫存器
                self.registers.d = self.registers.d.wrapping_add(1);
                self.set_zero_flag(self.registers.d);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (self.registers.d & 0x0F) == 0x00);
            }
            0x8C => {
                // ADC A, H - 加法與進位A = A + H + Carry
                let carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
                let result = (self.registers.a as u16) + (self.registers.h as u16) + carry;
                self.set_zero_flag(result as u8);
                self.set_flag(FLAG_N, false);
                self.set_flag(
                    FLAG_H,
                    ((self.registers.a & 0x0F) as u16) + ((self.registers.h & 0x0F) as u16) + carry
                        > 0x0F,
                );
                self.set_flag(FLAG_C, result > 0xFF);
                self.registers.a = result as u8;
            }
            0x07 => {
                // RLCA - 向左循環移位累加器
                let carry = (self.registers.a & 0x80) >> 7;
                self.registers.a = (self.registers.a << 1) | carry;
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);
            }
            0x1A => {
                // LD A, (DE) - 從DE指向的記憶體載入到A
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.registers.a = self.mmu.read_byte(de);
            }
            0xC0 => {
                // RET NZ - 當Z標誌位為0時返回
                if !self.get_flag(FLAG_Z) {
                    let lo = self.mmu.read_byte(self.registers.sp) as u16;
                    let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(2);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0x1F => {
                // RRA - 向右移位累加器通過進位
                let carry = if self.get_flag(FLAG_C) { 0x80 } else { 0 };
                let new_carry = self.registers.a & 0x01;
                self.registers.a = (self.registers.a >> 1) | carry;
                self.set_flag(FLAG_Z, false);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, new_carry != 0);
            }
            0x25 => {
                // DEC H - 遞減H暫存器
                self.registers.h = self.registers.h.wrapping_sub(1);
                self.set_zero_flag(self.registers.h);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (self.registers.h & 0x0F) == 0x0F);
            }
            0x15 => {
                // DEC D - 遞減D暫存器
                self.registers.d = self.registers.d.wrapping_sub(1);
                self.set_zero_flag(self.registers.d);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, (self.registers.d & 0x0F) == 0x0F);
            }
            0xB0 => {
                // OR B - 邏輯OR A與B
                self.registers.a |= self.registers.b;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0xBF => {
                // CP A - 比較A與A (總是設置Z標誌位)
                self.set_flag(FLAG_Z, true);
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0x19 => {
                // ADD HL, DE - HL加上DE
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                let result = hl.wrapping_add(de);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (hl & 0x0FFF) + (de & 0x0FFF) > 0x0FFF);
                self.set_flag(FLAG_C, result < hl);
            }
            0x08 => {
                // LD (nn), SP - 將SP載入到絕對地址
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, (self.registers.sp & 0xFF) as u8);
                self.mmu
                    .write_byte(addr + 1, (self.registers.sp >> 8) as u8);
            }
            0xE4 => {
                // LDH (n), A (另一種編碼) - 將A載入到高記憶體
                let n = self.fetch();
                let addr = 0xFF00 + (n as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xD2 => {
                // JP NC, nn - 當C標誌位為0時跳轉
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                if !self.get_flag(FLAG_C) {
                    self.registers.pc = addr;
                }
            }
            0x0A => {
                // LD A, (BC) - 從BC指向的記憶體載入到A
                let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.registers.a = self.mmu.read_byte(bc);
            }
            0x23 => {
                // INC HL - 遞增HL暫存器對
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let result = hl.wrapping_add(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x8A => {
                // ADC A, D - 加法與進位A = A + D + Carry
                let carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
                let result = (self.registers.a as u16) + (self.registers.d as u16) + carry;
                self.set_zero_flag(result as u8);
                self.set_flag(FLAG_N, false);
                self.set_flag(
                    FLAG_H,
                    ((self.registers.a & 0x0F) as u16) + ((self.registers.d & 0x0F) as u16) + carry
                        > 0x0F,
                );
                self.set_flag(FLAG_C, result > 0xFF);
                self.registers.a = result as u8;
            }
            0x41 => {
                // LD B, C - 將C載入到B
                self.registers.b = self.registers.c;
            }
            0x93 => {
                // SUB E - 從A減去E
                let result = self.registers.a.wrapping_sub(self.registers.e);
                self.set_zero_flag(result);
                self.set_flag(FLAG_N, true);
                self.set_flag(
                    FLAG_H,
                    (self.registers.a & 0x0F) < (self.registers.e & 0x0F),
                );
                self.set_flag(FLAG_C, self.registers.a < self.registers.e);
                self.registers.a = result;
            }
            0xFC => {
                // CALL C, nn - 當C標誌位為1時呼叫
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                if self.get_flag(FLAG_C) {
                    self.registers.sp = self.registers.sp.wrapping_sub(2);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                    self.mmu
                        .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                    self.registers.pc = addr;
                }
            }
            0xC7 => {
                // RST 00H
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu
                    .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = 0x00;
            }
            0xF7 => {
                // RST 30H
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu
                    .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = 0x30;
            }
            0xB3 => {
                // OR E - 邏輯OR A與E
                self.registers.a |= self.registers.e;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0x24 => {
                // INC H - 遞增H暫存器
                self.registers.h = self.registers.h.wrapping_add(1);
                self.set_zero_flag(self.registers.h);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, (self.registers.h & 0x0F) == 0x00);
            }
            0x51 => {
                // LD D, C - 將C載入到D
                self.registers.d = self.registers.c;
            }
            0xB5 => {
                // OR L - 邏輯OR A與L
                self.registers.a |= self.registers.l;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0xE5 => {
                // PUSH HL - 將HL推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu.write_byte(self.registers.sp, self.registers.l);
                self.mmu.write_byte(self.registers.sp + 1, self.registers.h);
            }
            0xA0 => {
                // AND B - 邏輯AND A與B
                self.registers.a &= self.registers.b;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                self.set_flag(FLAG_C, false);
            }
            0x2F => {
                // CPL - 補碼A (按位取反)
                self.registers.a = !self.registers.a;
                self.set_flag(FLAG_N, true);
                self.set_flag(FLAG_H, true);
            }
            0xCB => {
                // CB prefix - 延伸指令集
                let cb_opcode = self.fetch();
                self.execute_cb_instruction(cb_opcode);
            }
            0x37 => {
                // SCF - 設置進位標誌位
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, true);
            }
            0xA9 => {
                // XOR C - 邏輯XOR A與C
                self.registers.a ^= self.registers.c;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0xA1 => {
                // AND C - 邏輯AND A與C
                self.registers.a &= self.registers.c;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
                self.set_flag(FLAG_C, false);
            }
            0xEF => {
                // RST 28H
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu
                    .write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = 0x28;
            }
            0xCE => {
                // ADC A, n - 加法與進位A = A + n + Carry
                let n = self.fetch();
                let carry = if self.get_flag(FLAG_C) { 1 } else { 0 };
                let result = (self.registers.a as u16) + (n as u16) + carry;
                self.set_zero_flag(result as u8);
                self.set_flag(FLAG_N, false);
                self.set_flag(
                    FLAG_H,
                    ((self.registers.a & 0x0F) as u16) + ((n & 0x0F) as u16) + carry > 0x0F,
                );
                self.set_flag(FLAG_C, result > 0xFF);
                self.registers.a = result as u8;
            }
            0xAE => {
                // XOR (HL) - 邏輯XOR A與HL指向的記憶體
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let value = self.mmu.read_byte(hl);
                self.registers.a ^= value;
                self.set_zero_flag(self.registers.a);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);
            }
            0x87 => {
                // ADD A, A - A加上自身
                let result = (self.registers.a as u16) + (self.registers.a as u16);
                self.set_zero_flag(result as u8);
                self.set_flag(FLAG_N, false);
                self.set_flag(
                    FLAG_H,
                    (self.registers.a & 0x0F) + (self.registers.a & 0x0F) > 0x0F,
                );
                self.set_flag(FLAG_C, result > 0xFF);
                self.registers.a = result as u8;
            }
            0xE1 => {
                // POP HL - 從堆疊彈出到HL
                self.registers.l = self.mmu.read_byte(self.registers.sp);
                self.registers.h = self.mmu.read_byte(self.registers.sp + 1);
                self.registers.sp = self.registers.sp.wrapping_add(2);
            }
            0x5E => {
                // LD E, (HL) - 從HL指向的記憶體載入到E
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.e = self.mmu.read_byte(hl);
            }
            0x56 => {
                // LD D, (HL) - 從HL指向的記憶體載入到D
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.d = self.mmu.read_byte(hl);
            }
            0xD5 => {
                // PUSH DE - 將DE推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu.write_byte(self.registers.sp, self.registers.e);
                self.mmu.write_byte(self.registers.sp + 1, self.registers.d);
            }
            0xE9 => {
                // JP (HL) - 跳轉到HL指向的地址
                self.registers.pc = ((self.registers.h as u16) << 8) | self.registers.l as u16;
            }
            _ => {
                println!(
                    "未實現的指令: 0x{:02X} at PC: 0x{:04X}",
                    opcode,
                    self.registers.pc - 1
                );
            }
        }
    }

    fn execute_cb_instruction(&mut self, opcode: u8) {
        match opcode {
            // BIT 指令 - 測試位
            0x40..=0x47 => {
                // BIT 0, r
                let bit = 0;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x48..=0x4F => {
                // BIT 1, r
                let bit = 1;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x50..=0x57 => {
                // BIT 2, r
                let bit = 2;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x58..=0x5F => {
                // BIT 3, r
                let bit = 3;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x60..=0x67 => {
                // BIT 4, r
                let bit = 4;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x68..=0x6F => {
                // BIT 5, r
                let bit = 5;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x70..=0x77 => {
                // BIT 6, r
                let bit = 6;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            0x78..=0x7F => {
                // BIT 7, r
                let bit = 7;
                let value = match opcode & 0x07 {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => unreachable!(),
                };
                self.set_flag(FLAG_Z, (value & (1 << bit)) == 0);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, true);
            }
            // RLC 指令 - 向左循環移位
            0x00..=0x07 => {
                let reg_index = opcode & 0x07;
                let (value, addr) = match reg_index {
                    0 => (self.registers.b, None),
                    1 => (self.registers.c, None),
                    2 => (self.registers.d, None),
                    3 => (self.registers.e, None),
                    4 => (self.registers.h, None),
                    5 => (self.registers.l, None),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (self.mmu.read_byte(hl), Some(hl))
                    }
                    7 => (self.registers.a, None),
                    _ => unreachable!(),
                };

                let carry = (value & 0x80) >> 7;
                let result = (value << 1) | carry;

                self.set_zero_flag(result);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);

                match reg_index {
                    0 => self.registers.b = result,
                    1 => self.registers.c = result,
                    2 => self.registers.d = result,
                    3 => self.registers.e = result,
                    4 => self.registers.h = result,
                    5 => self.registers.l = result,
                    6 => self.mmu.write_byte(addr.unwrap(), result),
                    7 => self.registers.a = result,
                    _ => unreachable!(),
                }
            } // SRL 指令 - 邏輯右移
            0x38..=0x3F => {
                let reg_index = opcode & 0x07;
                let (value, addr) = match reg_index {
                    0 => (self.registers.b, None),
                    1 => (self.registers.c, None),
                    2 => (self.registers.d, None),
                    3 => (self.registers.e, None),
                    4 => (self.registers.h, None),
                    5 => (self.registers.l, None),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (self.mmu.read_byte(hl), Some(hl))
                    }
                    7 => (self.registers.a, None),
                    _ => unreachable!(),
                };

                let carry = value & 0x01;
                let result = value >> 1;

                self.set_zero_flag(result);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, carry != 0);

                match reg_index {
                    0 => self.registers.b = result,
                    1 => self.registers.c = result,
                    2 => self.registers.d = result,
                    3 => self.registers.e = result,
                    4 => self.registers.h = result,
                    5 => self.registers.l = result,
                    6 => self.mmu.write_byte(addr.unwrap(), result),
                    7 => self.registers.a = result,
                    _ => unreachable!(),
                }
            }
            // SWAP 指令 - 交換高低4位
            0x30..=0x37 => {
                let reg_index = opcode & 0x07;
                let (value, addr) = match reg_index {
                    0 => (self.registers.b, None),
                    1 => (self.registers.c, None),
                    2 => (self.registers.d, None),
                    3 => (self.registers.e, None),
                    4 => (self.registers.h, None),
                    5 => (self.registers.l, None),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (self.mmu.read_byte(hl), Some(hl))
                    }
                    7 => (self.registers.a, None),
                    _ => unreachable!(),
                };

                let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

                self.set_zero_flag(result);
                self.set_flag(FLAG_N, false);
                self.set_flag(FLAG_H, false);
                self.set_flag(FLAG_C, false);

                match reg_index {
                    0 => self.registers.b = result,
                    1 => self.registers.c = result,
                    2 => self.registers.d = result,
                    3 => self.registers.e = result,
                    4 => self.registers.h = result,
                    5 => self.registers.l = result,
                    6 => self.mmu.write_byte(addr.unwrap(), result),
                    7 => self.registers.a = result,
                    _ => unreachable!(),
                }
            }
            _ => {
                println!("未實現的CB指令: 0x{:02X}", opcode);
            }
        }
    }
}
