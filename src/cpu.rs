use crate::mmu::MMU;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Mutex;

lazy_static::lazy_static! {
    pub static ref UNIMPL_OPCODES: Mutex<HashSet<u8>> = Mutex::new(HashSet::new());
}

#[allow(dead_code)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
}
impl Registers {
    // Zero Flag (Z)
    pub fn get_z(&self) -> bool {
        self.f & 0x80 != 0
    }
    pub fn set_z(&mut self, val: bool) {
        if val {
            self.f |= 0x80;
        } else {
            self.f &= !0x80;
        }
    }
    // Subtract Flag (N)
    pub fn get_n(&self) -> bool {
        self.f & 0x40 != 0
    }
    pub fn set_n(&mut self, val: bool) {
        if val {
            self.f |= 0x40;
        } else {
            self.f &= !0x40;
        }
    }
    // Half Carry Flag (H)
    pub fn get_h(&self) -> bool {
        self.f & 0x20 != 0
    }
    pub fn set_h(&mut self, val: bool) {
        if val {
            self.f |= 0x20;
        } else {
            self.f &= !0x20;
        }
    }
    // Carry Flag (C)
    pub fn get_c(&self) -> bool {
        self.f & 0x10 != 0
    }
    pub fn set_c(&mut self, val: bool) {
        if val {
            self.f |= 0x10;
        } else {
            self.f &= !0x10;
        }
    }
    // 清除所有旗標
    #[allow(dead_code)]
    pub fn clear_flags(&mut self) {
        self.f = 0;
    }
}

pub struct CPU {
    pub registers: Registers,
    pub mmu: Rc<RefCell<MMU>>,
    pub ime: bool,     // Interrupt Master Enable flag
    pub halted: bool,  // CPU halted state
    pub stopped: bool, // CPU stopped state

    // 簡化的除錯屬性
    debug_enabled: bool,
}

impl CPU {
    // 將 read_byte 從私有方法改为公有方法
    #[allow(dead_code)]
    pub fn read_byte(&self, addr: u16) -> u8 {
        self.mmu.borrow().read_byte(addr)
    }

    /// 從 PC 讀取下一個字節並增加 PC
    fn fetch(&mut self) -> u8 {
        let value = self.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        value
    }

    /// Push a 16-bit value onto the stack (little-endian order)
    fn push_word(&mut self, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.mmu
            .borrow_mut()
            .write_byte(self.registers.sp, (value & 0xFF) as u8);
        self.mmu
            .borrow_mut()
            .write_byte(self.registers.sp + 1, (value >> 8) as u8);
    }

    #[allow(dead_code)]
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.mmu.borrow_mut().write_byte(addr, value)
    }

    #[allow(dead_code)]
    pub fn get_if(&self) -> u8 {
        self.mmu.borrow().if_reg
    }

    #[allow(dead_code)]
    pub fn get_ie(&self) -> u8 {
        self.mmu.borrow().ie_reg
    }

    /// 讀取 16 位值 (小端序)
    fn read_word(&self, addr: u16) -> u16 {
        let low = self.mmu.borrow().read_byte(addr) as u16;
        let high = self.mmu.borrow().read_byte(addr + 1) as u16;
        (high << 8) | low
    }

    /// 寫入 16 位值 (小端序)
    fn write_word(&mut self, addr: u16, value: u16) {
        self.mmu.borrow_mut().write_byte(addr, (value & 0xFF) as u8);
        self.mmu
            .borrow_mut()
            .write_byte(addr + 1, (value >> 8) as u8);
    }

    /// 從堆疊彈出 16 位值
    fn pop_word(&mut self) -> u16 {
        let value = self.read_word(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(2);
        value
    }

    #[allow(dead_code)]
    fn handle_interrupts(&mut self) {
        // 將所有需要的值複製到局部變數，避免借用衝突
        let ie_reg;
        let if_reg;
        {
            let mmu_ref = self.mmu.borrow();
            ie_reg = mmu_ref.ie_reg;
            if_reg = mmu_ref.if_reg;
        }

        let pending = ie_reg & if_reg;

        if self.ime && pending != 0 {
            // 有待處理的中斷
            self.ime = false; // 禁用中斷
            self.halted = false; // 結束 HALT 狀態

            let interrupt_addr;
            let interrupt_flag;

            // 按優先級處理中斷
            if pending & 0x01 != 0 {
                interrupt_addr = 0x40; // VBlank
                interrupt_flag = 0x01;
            } else if pending & 0x02 != 0 {
                interrupt_addr = 0x48; // LCD STAT
                interrupt_flag = 0x02;
            } else if pending & 0x04 != 0 {
                interrupt_addr = 0x50; // Timer
                interrupt_flag = 0x04;
            } else if pending & 0x08 != 0 {
                interrupt_addr = 0x58; // Serial
                interrupt_flag = 0x08;
            } else if pending & 0x10 != 0 {
                interrupt_addr = 0x60; // Joypad
                interrupt_flag = 0x10;
            } else {
                return; // 沒有中斷需要處理
            }

            // 清除 IF 中的對應標誌
            {
                let mut mmu_mut = self.mmu.borrow_mut();
                mmu_mut.if_reg &= !interrupt_flag;
            }

            // 推送 PC 到堆疊，並跳轉到中斷處理例程
            self.push_word(self.registers.pc);
            self.registers.pc = interrupt_addr;

            return;
        }

        // 當 HALT 狀態有中斷請求時，結束 HALT
        if self.halted && pending != 0 {
            self.halted = false;
        }
    }
    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
            ime: false,
            halted: false,
            stopped: false,
            debug_enabled: false,
        }
    }

    /// 啟用或禁用除錯模式
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_enabled = enabled;
        println!("CPU 除錯模式 {}", if enabled { "已啟用" } else { "已禁用" });
    }
    pub fn step(&mut self) -> u8 {
        // 先檢查中斷
        self.handle_interrupts();

        // 如果 CPU 處於 HALT 狀態
        if self.halted {
            return 4; // HALT 期間每個步驟消耗 4 個週期
        }

        // 取得指令
        let opcode = self.fetch();

        // 執行指令
        let cycles = self.decode_and_execute(opcode);
        cycles
    }

    /// 輸出一段記憶體區域的內容
    pub fn dump_memory(&self, start: u16, end: u16) {
        if !self.debug_enabled {
            return;
        }

        println!("=== 記憶體轉儲 (0x{:04X} - 0x{:04X}) ===", start, end);

        for addr in (start..=end).step_by(16) {
            let mut line = format!("{:04X}:", addr);

            for offset in 0..16 {
                if addr + offset <= end {
                    line.push_str(&format!(" {:02X}", self.read_byte(addr + offset)));
                }
            }

            println!("{}", line);
        }
    }

    /// 解析執行計劃，返回未來將執行的指令序列
    pub fn peek_ahead(&self, steps: usize) -> Vec<(u16, u8, String)> {
        let mut result = Vec::with_capacity(steps);
        let mut pc = self.registers.pc;

        for _ in 0..steps {
            let opcode = self.read_byte(pc);
            let disasm = self.disassemble_instruction(pc, opcode);
            result.push((pc, opcode, disasm));

            // 預測 PC 會如何變化
            pc = match opcode {
                0xC3 => {
                    // JP nn
                    let low = self.read_byte(pc + 1) as u16;
                    let high = self.read_byte(pc + 2) as u16;
                    (high << 8) | low
                }
                0x18 => {
                    // JR n
                    let offset = self.read_byte(pc + 1) as i8;
                    pc.wrapping_add(2).wrapping_add(offset as u16)
                }
                0xC9 => {
                    // RET
                    // 這裡我們不知道堆疊上有什麼值，因此我們無法準確預測
                    // 簡單返回一個特殊值表示無法預測
                    0xFFFF
                }
                _ => {
                    // 簡單情況: 指令長度等於 1, 2, 或 3 字節
                    let instr_len = match opcode {
                        0xCB => 2,                                                  // CB 前綴指令
                        0x01 | 0x11 | 0x21 | 0x31 | 0x08 | 0xC3 | 0xCD => 3,        // 3 字節指令
                        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => 2, // 2 字節指令
                        0x20 | 0x28 | 0x30 | 0x38 => 2,                             // 條件跳轉指令
                        _ => 1,                                                     // 默認為 1 字節
                    };
                    pc.wrapping_add(instr_len)
                }
            };

            // 如果我們無法預測下一個 PC，就停止預測
            if pc == 0xFFFF {
                break;
            }
        }

        result
    }

    /// 產生 CPU 狀態的詳細報告
    pub fn generate_detailed_report(&self) -> String {
        let mut report = String::new();

        report.push_str("==== CPU 詳細報告 ====\n");

        // 基本 CPU 狀態
        report.push_str(&format!(
            "PC: 0x{:04X} SP: 0x{:04X}\n",
            self.registers.pc, self.registers.sp
        ));
        report.push_str(&format!(
            "A: 0x{:02X} F: 0x{:02X} (Z:{} N:{} H:{} C:{})\n",
            self.registers.a,
            self.registers.f,
            self.registers.get_z() as u8,
            self.registers.get_n() as u8,
            self.registers.get_h() as u8,
            self.registers.get_c() as u8
        ));
        report.push_str(&format!(
            "B: 0x{:02X} C: 0x{:02X} D: 0x{:02X} E: 0x{:02X}\n",
            self.registers.b, self.registers.c, self.registers.d, self.registers.e
        ));
        report.push_str(&format!(
            "H: 0x{:02X} L: 0x{:02X}\n",
            self.registers.h, self.registers.l
        ));
        report.push_str(&format!(
            "BC: 0x{:04X} DE: 0x{:04X} HL: 0x{:04X}\n\n",
            ((self.registers.b as u16) << 8) | (self.registers.c as u16),
            ((self.registers.d as u16) << 8) | (self.registers.e as u16),
            ((self.registers.h as u16) << 8) | (self.registers.l as u16)
        ));

        // 中斷狀態
        report.push_str(&format!(
            "IME: {} HALT: {} STOP: {}\n",
            self.ime, self.halted, self.stopped
        ));
        report.push_str(&format!(
            "IF: 0x{:02X} IE: 0x{:02X}\n\n",
            self.get_if(),
            self.get_ie()
        ));

        // 當前指令
        let pc = self.registers.pc;
        let opcode = self.read_byte(pc);
        let disasm = self.disassemble_instruction(pc, opcode);
        report.push_str(&format!(
            "當前指令: {:04X}: {:02X} {}\n\n",
            pc, opcode, disasm
        ));

        // 堆疊內容
        report.push_str("堆疊頂部:\n");
        for offset in 0..8 {
            let addr = self.registers.sp.wrapping_add(offset * 2);
            if addr < 0xFFFE {
                let value = self.read_word(addr);
                report.push_str(&format!(
                    "SP+{:02X} ({:04X}): {:04X}\n",
                    offset * 2,
                    addr,
                    value
                ));
            }
        } // 記憶體檢視
        report.push_str("\n記憶體位置 @ PC:\n");
        for offset in -8i16..8i16 {
            let addr = pc.wrapping_add(offset as u16);
            let value = self.read_byte(addr);
            report.push_str(&format!(
                "{:04X}: {:02X} {}\n",
                addr,
                value,
                if offset == 0 { "<<< PC" } else { "" }
            ));
        }

        report
    }

    /// 執行單步除錯，輸出詳細說明
    /// 解碼並執行指令
    fn decode_and_execute(&mut self, opcode: u8) -> u8 {
        match opcode {
            // 0x00-0x0F
            0x00 => 4, // NOP
            0x01 => {
                // LD BC,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.registers.b = (nn >> 8) as u8;
                self.registers.c = (nn & 0xFF) as u8;
                12
            }
            0x02 => {
                // LD (BC),A
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.write_byte(bc, self.registers.a);
                8
            }
            0x03 => {
                // INC BC
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_add(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = (result & 0xFF) as u8;
                8
            }
            0x04 => {
                // INC B
                let result = self.registers.b.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.b & 0x0F) == 0x0F);
                self.registers.b = result;
                4
            }
            0x05 => {
                // DEC B
                let result = self.registers.b.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.b & 0x0F) == 0);
                self.registers.b = result;
                4
            }
            0x06 => {
                // LD B,n
                let n = self.fetch();
                self.registers.b = n;
                8
            }
            0x07 => {
                // RLCA
                let carry = (self.registers.a & 0x80) != 0;
                self.registers.a = self.registers.a.rotate_left(1);
                self.registers.set_z(false);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
                4
            }
            0x08 => {
                // LD (nn),SP
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.write_word(nn, self.registers.sp);
                20
            }
            0x09 => {
                // ADD HL,BC
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let (result, carry) = hl.overflowing_add(bc);
                let half_carry = (hl & 0x0FFF) + (bc & 0x0FFF) > 0x0FFF;

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                self.registers.set_n(false);
                self.registers.set_h(half_carry);
                self.registers.set_c(carry);
                8
            }
            0x0A => {
                // LD A,(BC)
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.registers.a = self.read_byte(bc);
                8
            }
            0x0B => {
                // DEC BC
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_sub(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = (result & 0xFF) as u8;
                8
            }
            0x0C => {
                // INC C
                let result = self.registers.c.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.c & 0x0F) == 0x0F);
                self.registers.c = result;
                4
            }
            0x0D => {
                // DEC C
                let result = self.registers.c.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.c & 0x0F) == 0);
                self.registers.c = result;
                4
            }
            0x0E => {
                // LD C,n
                let n = self.fetch();
                self.registers.c = n;
                8
            }
            0x0F => {
                // RRCA
                let carry = (self.registers.a & 0x01) != 0;
                self.registers.a = self.registers.a.rotate_right(1);
                self.registers.set_z(false);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
                4
            }

            // 0x10-0x1F
            0x10 => {
                // STOP
                self.stopped = true;
                let _next_byte = self.fetch(); // STOP 指令後跟一個 00 字節
                4
            }
            0x11 => {
                // LD DE,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.registers.d = (nn >> 8) as u8;
                self.registers.e = (nn & 0xFF) as u8;
                12
            }
            0x12 => {
                // LD (DE),A
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.write_byte(de, self.registers.a);
                8
            }
            0x13 => {
                // INC DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = de.wrapping_add(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = (result & 0xFF) as u8;
                8
            }
            0x14 => {
                // INC D
                let result = self.registers.d.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.d & 0x0F) == 0x0F);
                self.registers.d = result;
                4
            }
            0x15 => {
                // DEC D
                let result = self.registers.d.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.d & 0x0F) == 0);
                self.registers.d = result;
                4
            }
            0x16 => {
                // LD D,n
                let n = self.fetch();
                self.registers.d = n;
                8
            }
            0x17 => {
                // RLA
                let old_carry = self.registers.get_c();
                let new_carry = (self.registers.a & 0x80) != 0;
                self.registers.a = (self.registers.a << 1) | if old_carry { 1 } else { 0 };
                self.registers.set_z(false);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(new_carry);
                4
            }
            0x18 => {
                // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
                12
            }
            0x19 => {
                // ADD HL,DE
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let (result, carry) = hl.overflowing_add(de);
                let half_carry = (hl & 0x0FFF) + (de & 0x0FFF) > 0x0FFF;

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                self.registers.set_n(false);
                self.registers.set_h(half_carry);
                self.registers.set_c(carry);
                8
            }
            0x1A => {
                // LD A,(DE)
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.registers.a = self.read_byte(de);
                8
            }
            0x1B => {
                // DEC DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = de.wrapping_sub(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = (result & 0xFF) as u8;
                8
            }
            0x1C => {
                // INC E
                let result = self.registers.e.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.e & 0x0F) == 0x0F);
                self.registers.e = result;
                4
            }
            0x1D => {
                // DEC E
                let result = self.registers.e.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.e & 0x0F) == 0);
                self.registers.e = result;
                4
            }
            0x1E => {
                // LD E,n
                let n = self.fetch();
                self.registers.e = n;
                8
            }
            0x1F => {
                // RRA
                let old_carry = self.registers.get_c();
                let new_carry = (self.registers.a & 0x01) != 0;
                self.registers.a = (self.registers.a >> 1) | if old_carry { 0x80 } else { 0 };
                self.registers.set_z(false);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(new_carry);
                4
            }

            // 0x20-0x2F (包含條件分支)
            0x20 => {
                // JR NZ,n
                let offset = self.fetch() as i8;
                if !self.registers.get_z() {
                    self.registers.pc =
                        (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
                    12 // 分支發生
                } else {
                    8 // 分支未發生
                }
            }
            0x21 => {
                // LD HL,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.registers.h = (nn >> 8) as u8;
                self.registers.l = (nn & 0xFF) as u8;
                12
            }
            0x22 => {
                // LD (HL+),A
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.write_byte(hl, self.registers.a);
                let new_hl = hl.wrapping_add(1);
                self.registers.h = (new_hl >> 8) as u8;
                self.registers.l = (new_hl & 0xFF) as u8;
                8
            }
            0x23 => {
                // INC HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let result = hl.wrapping_add(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                8
            }
            0x24 => {
                // INC H
                let result = self.registers.h.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.h & 0x0F) == 0x0F);
                self.registers.h = result;
                4
            }
            0x25 => {
                // DEC H
                let result = self.registers.h.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.h & 0x0F) == 0);
                self.registers.h = result;
                4
            }
            0x26 => {
                // LD H,n
                let n = self.fetch();
                self.registers.h = n;
                8
            }
            0x27 => {
                // DAA (Decimal Adjust A)
                let mut a = self.registers.a;
                let mut carry = false;

                if !self.registers.get_n() {
                    // Addition
                    if self.registers.get_c() || a > 0x99 {
                        a = a.wrapping_add(0x60);
                        carry = true;
                    }
                    if self.registers.get_h() || (a & 0x0F) > 0x09 {
                        a = a.wrapping_add(0x06);
                    }
                } else {
                    // Subtraction
                    if self.registers.get_c() {
                        a = a.wrapping_sub(0x60);
                        carry = true;
                    }
                    if self.registers.get_h() {
                        a = a.wrapping_sub(0x06);
                    }
                }

                self.registers.a = a;
                self.registers.set_z(a == 0);
                self.registers.set_h(false);
                self.registers.set_c(carry);
                4
            }
            0x28 => {
                // JR Z,n
                let offset = self.fetch() as i8;
                if self.registers.get_z() {
                    self.registers.pc =
                        (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
                    12 // 分支發生
                } else {
                    8 // 分支未發生
                }
            }
            0x29 => {
                // ADD HL,HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let (result, carry) = hl.overflowing_add(hl);
                let half_carry = (hl & 0x0FFF) + (hl & 0x0FFF) > 0x0FFF;

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                self.registers.set_n(false);
                self.registers.set_h(half_carry);
                self.registers.set_c(carry);
                8
            }
            0x2A => {
                // LD A,(HL+)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.read_byte(hl);
                let new_hl = hl.wrapping_add(1);
                self.registers.h = (new_hl >> 8) as u8;
                self.registers.l = (new_hl & 0xFF) as u8;
                8
            }
            0x2B => {
                // DEC HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let result = hl.wrapping_sub(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                8
            }
            0x2C => {
                // INC L
                let result = self.registers.l.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.l & 0x0F) == 0x0F);
                self.registers.l = result;
                4
            }
            0x2D => {
                // DEC L
                let result = self.registers.l.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.l & 0x0F) == 0);
                self.registers.l = result;
                4
            }
            0x2E => {
                // LD L,n
                let n = self.fetch();
                self.registers.l = n;
                8
            }
            0x2F => {
                // CPL (Complement A)
                self.registers.a = !self.registers.a;
                self.registers.set_n(true);
                self.registers.set_h(true);
                4
            }

            // 0x30-0x3F
            0x30 => {
                // JR NC,n
                let offset = self.fetch() as i8;
                if !self.registers.get_c() {
                    self.registers.pc =
                        (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
                    12 // 分支發生
                } else {
                    8 // 分支未發生
                }
            }
            0x31 => {
                // LD SP,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.registers.sp = nn;
                12
            }
            0x32 => {
                // LD (HL-),A
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.write_byte(hl, self.registers.a);
                let new_hl = hl.wrapping_sub(1);
                self.registers.h = (new_hl >> 8) as u8;
                self.registers.l = (new_hl & 0xFF) as u8;
                8
            }
            0x33 => {
                // INC SP
                self.registers.sp = self.registers.sp.wrapping_add(1);
                8
            }
            0x34 => {
                // INC (HL)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let value = self.read_byte(hl);
                let result = value.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((value & 0x0F) == 0x0F);
                self.write_byte(hl, result);
                12
            }
            0x35 => {
                // DEC (HL)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let value = self.read_byte(hl);
                let result = value.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((value & 0x0F) == 0);
                self.write_byte(hl, result);
                12
            }
            0x36 => {
                // LD (HL),n
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let n = self.fetch();
                self.write_byte(hl, n);
                12
            }
            0x37 => {
                // SCF (Set Carry Flag)
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(true);
                4
            }
            0x38 => {
                // JR C,n
                let offset = self.fetch() as i8;
                if self.registers.get_c() {
                    self.registers.pc =
                        (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
                    12 // 分支發生
                } else {
                    8 // 分支未發生
                }
            }
            0x39 => {
                // ADD HL,SP
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let (result, carry) = hl.overflowing_add(self.registers.sp);
                let half_carry = (hl & 0x0FFF) + (self.registers.sp & 0x0FFF) > 0x0FFF;

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                self.registers.set_n(false);
                self.registers.set_h(half_carry);
                self.registers.set_c(carry);
                8
            }
            0x3A => {
                // LD A,(HL-)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.read_byte(hl);
                let new_hl = hl.wrapping_sub(1);
                self.registers.h = (new_hl >> 8) as u8;
                self.registers.l = (new_hl & 0xFF) as u8;
                8
            }
            0x3B => {
                // DEC SP
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                8
            }
            0x3C => {
                // INC A
                let result = self.registers.a.wrapping_add(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h((self.registers.a & 0x0F) == 0x0F);
                self.registers.a = result;
                4
            }
            0x3D => {
                // DEC A
                let result = self.registers.a.wrapping_sub(1);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((self.registers.a & 0x0F) == 0);
                self.registers.a = result;
                4
            }
            0x3E => {
                // LD A,n
                let n = self.fetch();
                self.registers.a = n;
                8
            }
            0x3F => {
                // CCF (Complement Carry Flag)
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(!self.registers.get_c());
                4
            }

            // 0x40-0x7F: LD r,r' 指令組 (寄存器間載入)
            0x40..=0x7F => {
                let dst = (opcode >> 3) & 0x07;
                let src = opcode & 0x07;

                if opcode == 0x76 {
                    // HALT 指令
                    self.halted = true;
                    4
                } else {
                    // LD r,r'
                    let value = self.get_register_value(src);
                    self.set_register_value(dst, value);
                    if src == 6 || dst == 6 {
                        8 // 涉及 (HL) 的指令需要 8 個週期
                    } else {
                        4 // 寄存器間載入需要 4 個週期
                    }
                }
            }

            // 0x80-0x87: ADD A,r
            0x80..=0x87 => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let (result, carry) = self.registers.a.overflowing_add(value);
                let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(half_carry);
                self.registers.set_c(carry);
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0x88-0x8F: ADC A,r
            0x88..=0x8F => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let carry_in = if self.registers.get_c() { 1 } else { 0 };
                let temp = (self.registers.a as u16) + (value as u16) + carry_in;
                let result = temp as u8;

                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers
                    .set_h((self.registers.a & 0x0F) + (value & 0x0F) + carry_in as u8 > 0x0F);
                self.registers.set_c(temp > 0xFF);
                self.registers.a = result;
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0x90-0x97: SUB A,r
            0x90..=0x97 => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let (result, borrow) = self.registers.a.overflowing_sub(value);
                let half_borrow = (self.registers.a & 0x0F) < (value & 0x0F);

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h(half_borrow);
                self.registers.set_c(borrow);
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0x98-0x9F: SBC A,r
            0x98..=0x9F => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let carry_in = if self.registers.get_c() { 1 } else { 0 };
                let temp = (self.registers.a as i16) - (value as i16) - (carry_in as i16);
                let result = temp as u8;

                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers
                    .set_h((self.registers.a & 0x0F) < (value & 0x0F) + carry_in);
                self.registers.set_c(temp < 0);
                self.registers.a = result;
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0xA0-0xA7: AND A,r
            0xA0..=0xA7 => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let result = self.registers.a & value;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(true);
                self.registers.set_c(false);
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0xA8-0xAF: XOR A,r
            0xA8..=0xAF => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let result = self.registers.a ^ value;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(false);
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0xB0-0xB7: OR A,r
            0xB0..=0xB7 => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let result = self.registers.a | value;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(false);
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0xB8-0xBF: CP A,r
            0xB8..=0xBF => {
                let reg = opcode & 0x07;
                let value = self.get_register_value(reg);
                let (result, borrow) = self.registers.a.overflowing_sub(value);
                let half_borrow = (self.registers.a & 0x0F) < (value & 0x0F);

                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h(half_borrow);
                self.registers.set_c(borrow);
                if reg == 6 {
                    8
                } else {
                    4
                }
            }

            // 0xC0-0xFF: 控制流和其他指令
            0xC0 => {
                // RET NZ
                if !self.registers.get_z() {
                    self.registers.pc = self.pop_word();
                    20
                } else {
                    8
                }
            }
            0xC1 => {
                // POP BC
                let value = self.pop_word();
                self.registers.b = (value >> 8) as u8;
                self.registers.c = (value & 0xFF) as u8;
                12
            }
            0xC2 => {
                // JP NZ,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if !self.registers.get_z() {
                    self.registers.pc = nn;
                    16
                } else {
                    12
                }
            }
            0xC3 => {
                // JP nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = nn;
                16
            }
            0xC4 => {
                // CALL NZ,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if !self.registers.get_z() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = nn;
                    24
                } else {
                    12
                }
            }
            0xC5 => {
                // PUSH BC
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.push_word(bc);
                16
            }
            0xC6 => {
                // ADD A,n
                let n = self.fetch();
                let (result, carry) = self.registers.a.overflowing_add(n);
                let half_carry = (self.registers.a & 0x0F) + (n & 0x0F) > 0x0F;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(half_carry);
                self.registers.set_c(carry);
                8
            }
            0xC7 => {
                // RST 00H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x00;
                16
            }
            0xC8 => {
                // RET Z
                if self.registers.get_z() {
                    self.registers.pc = self.pop_word();
                    20
                } else {
                    8
                }
            }
            0xC9 => {
                // RET
                self.registers.pc = self.pop_word();
                16
            }
            0xCA => {
                // JP Z,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if self.registers.get_z() {
                    self.registers.pc = nn;
                    16
                } else {
                    12
                }
            }
            0xCB => {
                // CB 前綴指令
                let cb_opcode = self.fetch();
                self.execute_cb_instruction(cb_opcode)
            }
            0xCC => {
                // CALL Z,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if self.registers.get_z() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = nn;
                    24
                } else {
                    12
                }
            }
            0xCD => {
                // CALL nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.push_word(self.registers.pc);
                self.registers.pc = nn;
                24
            }
            0xCE => {
                // ADC A,n
                let n = self.fetch();
                let carry_in = if self.registers.get_c() { 1 } else { 0 };
                let temp = (self.registers.a as u16) + (n as u16) + carry_in;
                let result = temp as u8;

                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers
                    .set_h((self.registers.a & 0x0F) + (n & 0x0F) + carry_in as u8 > 0x0F);
                self.registers.set_c(temp > 0xFF);
                self.registers.a = result;
                8
            }
            0xCF => {
                // RST 08H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x08;
                16
            }

            0xD0 => {
                // RET NC
                if !self.registers.get_c() {
                    self.registers.pc = self.pop_word();
                    20
                } else {
                    8
                }
            }
            0xD1 => {
                // POP DE
                let value = self.pop_word();
                self.registers.d = (value >> 8) as u8;
                self.registers.e = (value & 0xFF) as u8;
                12
            }
            0xD2 => {
                // JP NC,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if !self.registers.get_c() {
                    self.registers.pc = nn;
                    16
                } else {
                    12
                }
            }
            0xD4 => {
                // CALL NC,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if !self.registers.get_c() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = nn;
                    24
                } else {
                    12
                }
            }
            0xD5 => {
                // PUSH DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.push_word(de);
                16
            }
            0xD6 => {
                // SUB A,n
                let n = self.fetch();
                let (result, borrow) = self.registers.a.overflowing_sub(n);
                let half_borrow = (self.registers.a & 0x0F) < (n & 0x0F);

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h(half_borrow);
                self.registers.set_c(borrow);
                8
            }
            0xD7 => {
                // RST 10H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x10;
                16
            }
            0xD8 => {
                // RET C
                if self.registers.get_c() {
                    self.registers.pc = self.pop_word();
                    20
                } else {
                    8
                }
            }
            0xD9 => {
                // RETI
                self.registers.pc = self.pop_word();
                self.ime = true;
                16
            }
            0xDA => {
                // JP C,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if self.registers.get_c() {
                    self.registers.pc = nn;
                    16
                } else {
                    12
                }
            }
            0xDC => {
                // CALL C,nn
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                if self.registers.get_c() {
                    self.push_word(self.registers.pc);
                    self.registers.pc = nn;
                    24
                } else {
                    12
                }
            }
            0xDE => {
                // SBC A,n
                let n = self.fetch();
                let carry_in = if self.registers.get_c() { 1 } else { 0 };
                let temp = (self.registers.a as i16) - (n as i16) - (carry_in as i16);
                let result = temp as u8;

                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers
                    .set_h((self.registers.a & 0x0F) < (n & 0x0F) + carry_in);
                self.registers.set_c(temp < 0);
                self.registers.a = result;
                8
            }
            0xDF => {
                // RST 18H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x18;
                16
            }

            0xE0 => {
                // LD (FF00+n),A
                let n = self.fetch();
                self.write_byte(0xFF00 + n as u16, self.registers.a);
                12
            }
            0xE1 => {
                // POP HL
                let value = self.pop_word();
                self.registers.h = (value >> 8) as u8;
                self.registers.l = (value & 0xFF) as u8;
                12
            }
            0xE2 => {
                // LD (FF00+C),A
                self.write_byte(0xFF00 + self.registers.c as u16, self.registers.a);
                8
            }
            0xE5 => {
                // PUSH HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.push_word(hl);
                16
            }
            0xE6 => {
                // AND n
                let n = self.fetch();
                let result = self.registers.a & n;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(true);
                self.registers.set_c(false);
                8
            }
            0xE7 => {
                // RST 20H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x20;
                16
            }
            0xE8 => {
                // ADD SP,n
                let n = self.fetch() as i8;
                let sp = self.registers.sp;
                let result = sp.wrapping_add(n as u16);

                self.registers.set_z(false);
                self.registers.set_n(false);
                self.registers
                    .set_h((sp & 0x0F) + ((n as u16) & 0x0F) > 0x0F);
                self.registers
                    .set_c((sp & 0xFF) + ((n as u16) & 0xFF) > 0xFF);
                self.registers.sp = result;
                16
            }
            0xE9 => {
                // JP (HL)
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.pc = hl;
                4
            }
            0xEA => {
                // LD (nn),A
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.write_byte(nn, self.registers.a);
                16
            }
            0xEE => {
                // XOR n
                let n = self.fetch();
                let result = self.registers.a ^ n;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(false);
                8
            }
            0xEF => {
                // RST 28H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x28;
                16
            }

            0xF0 => {
                // LD A,(FF00+n)
                let n = self.fetch();
                self.registers.a = self.read_byte(0xFF00 + n as u16);
                12
            }
            0xF1 => {
                // POP AF
                let value = self.pop_word();
                self.registers.a = (value >> 8) as u8;
                self.registers.f = value as u8 & 0xF0; // 只有高 4 位有效
                12
            }
            0xF2 => {
                // LD A,(FF00+C)
                self.registers.a = self.read_byte(0xFF00 + self.registers.c as u16);
                8
            }
            0xF3 => {
                // DI (Disable Interrupts)
                self.ime = false;
                4
            }
            0xF5 => {
                // PUSH AF
                let af = ((self.registers.a as u16) << 8) | (self.registers.f as u16);
                self.push_word(af);
                16
            }
            0xF6 => {
                // OR n
                let n = self.fetch();
                let result = self.registers.a | n;

                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(false);
                8
            }
            0xF7 => {
                // RST 30H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x30;
                16
            }
            0xF8 => {
                // LD HL,SP+n
                let n = self.fetch() as i8;
                let sp = self.registers.sp;
                let result = sp.wrapping_add(n as u16);

                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
                self.registers.set_z(false);
                self.registers.set_n(false);
                self.registers
                    .set_h((sp & 0x0F) + ((n as u16) & 0x0F) > 0x0F);
                self.registers
                    .set_c((sp & 0xFF) + ((n as u16) & 0xFF) > 0xFF);
                12
            }
            0xF9 => {
                // LD SP,HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.sp = hl;
                8
            }
            0xFA => {
                // LD A,(nn)
                let nn = self.read_word(self.registers.pc);
                self.registers.pc = self.registers.pc.wrapping_add(2);
                self.registers.a = self.read_byte(nn);
                16
            }
            0xFB => {
                // EI (Enable Interrupts)
                self.ime = true;
                4
            }
            0xFE => {
                // CP n
                let n = self.fetch();
                let (result, borrow) = self.registers.a.overflowing_sub(n);
                let half_borrow = (self.registers.a & 0x0F) < (n & 0x0F);

                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h(half_borrow);
                self.registers.set_c(borrow);
                8
            }
            0xFF => {
                // RST 38H
                self.push_word(self.registers.pc);
                self.registers.pc = 0x38;
                16
            }

            // 未實現或無效的指令
            _ => {
                let mut unimpl = UNIMPL_OPCODES.lock().unwrap();
                unimpl.insert(opcode);
                if unimpl.len() <= 10 {
                    println!(
                        "警告: 未實現的指令 0x{:02X} at PC: 0x{:04X}",
                        opcode,
                        self.registers.pc.wrapping_sub(1)
                    );
                }
                4 // 默認週期數
            }
        }
    }

    /// 執行 CB 前綴指令
    fn execute_cb_instruction(&mut self, cb_opcode: u8) -> u8 {
        match cb_opcode {
            // RLC 指令 (0x00-0x07)
            0x00 => {
                self.registers.b = self.rlc(self.registers.b);
                8
            }
            0x01 => {
                self.registers.c = self.rlc(self.registers.c);
                8
            }
            0x02 => {
                self.registers.d = self.rlc(self.registers.d);
                8
            }
            0x03 => {
                self.registers.e = self.rlc(self.registers.e);
                8
            }
            0x04 => {
                self.registers.h = self.rlc(self.registers.h);
                8
            }
            0x05 => {
                self.registers.l = self.rlc(self.registers.l);
                8
            }
            0x06 => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.rlc(val);
                self.write_byte(hl, result);
                16
            }
            0x07 => {
                self.registers.a = self.rlc(self.registers.a);
                8
            }

            // RRC 指令 (0x08-0x0F)
            0x08 => {
                self.registers.b = self.rrc(self.registers.b);
                8
            }

            0x09 => {
                self.registers.c = self.rrc(self.registers.c);
                8
            }
            0x0A => {
                self.registers.d = self.rrc(self.registers.d);
                8
            }
            0x0B => {
                self.registers.e = self.rrc(self.registers.e);
                8
            }
            0x0C => {
                self.registers.h = self.rrc(self.registers.h);
                8
            }
            0x0D => {
                self.registers.l = self.rrc(self.registers.l);
                8
            }
            0x0E => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.rrc(val);
                self.write_byte(hl, result);
                16
            }
            0x0F => {
                self.registers.a = self.rrc(self.registers.a);
                8
            }

            // RL 指令 (0x10-0x17)
            0x10 => {
                self.registers.b = self.rl(self.registers.b);
                8
            }
            0x11 => {
                self.registers.c = self.rl(self.registers.c);
                8
            }
            0x12 => {
                self.registers.d = self.rl(self.registers.d);
                8
            }
            0x13 => {
                self.registers.e = self.rl(self.registers.e);
                8
            }
            0x14 => {
                self.registers.h = self.rl(self.registers.h);
                8
            }
            0x15 => {
                self.registers.l = self.rl(self.registers.l);
                8
            }
            0x16 => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.rl(val);
                self.write_byte(hl, result);
                16
            }
            0x17 => {
                self.registers.a = self.rl(self.registers.a);
                8
            }

            // RR 指令 (0x18-0x1F)
            0x18 => {
                self.registers.b = self.rr(self.registers.b);
                8
            }
            0x19 => {
                self.registers.c = self.rr(self.registers.c);
                8
            }
            0x1A => {
                self.registers.d = self.rr(self.registers.d);
                8
            }
            0x1B => {
                self.registers.e = self.rr(self.registers.e);
                8
            }
            0x1C => {
                self.registers.h = self.rr(self.registers.h);
                8
            }
            0x1D => {
                self.registers.l = self.rr(self.registers.l);
                8
            }
            0x1E => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.rr(val);
                self.write_byte(hl, result);
                16
            }
            0x1F => {
                self.registers.a = self.rr(self.registers.a);
                8
            }

            // SLA 指令 (0x20-0x27)
            0x20 => {
                self.registers.b = self.sla(self.registers.b);
                8
            }
            0x21 => {
                self.registers.c = self.sla(self.registers.c);
                8
            }
            0x22 => {
                self.registers.d = self.sla(self.registers.d);
                8
            }
            0x23 => {
                self.registers.e = self.sla(self.registers.e);
                8
            }
            0x24 => {
                self.registers.h = self.sla(self.registers.h);
                8
            }
            0x25 => {
                self.registers.l = self.sla(self.registers.l);
                8
            }
            0x26 => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.sla(val);
                self.write_byte(hl, result);
                16
            }
            0x27 => {
                self.registers.a = self.sla(self.registers.a);
                8
            }

            // SRA 指令 (0x28-0x2F)
            0x28 => {
                self.registers.b = self.sra(self.registers.b);
                8
            }
            0x29 => {
                self.registers.c = self.sra(self.registers.c);
                8
            }
            0x2A => {
                self.registers.d = self.sra(self.registers.d);
                8
            }
            0x2B => {
                self.registers.e = self.sra(self.registers.e);
                8
            }
            0x2C => {
                self.registers.h = self.sra(self.registers.h);
                8
            }
            0x2D => {
                self.registers.l = self.sra(self.registers.l);
                8
            }
            0x2E => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.sra(val);
                self.write_byte(hl, result);
                16
            }
            0x2F => {
                self.registers.a = self.sra(self.registers.a);
                8
            }

            // SWAP 指令 (0x30-0x37)
            0x30 => {
                self.registers.b = self.swap(self.registers.b);
                8
            }
            0x31 => {
                self.registers.c = self.swap(self.registers.c);
                8
            }
            0x32 => {
                self.registers.d = self.swap(self.registers.d);
                8
            }
            0x33 => {
                self.registers.e = self.swap(self.registers.e);
                8
            }
            0x34 => {
                self.registers.h = self.swap(self.registers.h);
                8
            }
            0x35 => {
                self.registers.l = self.swap(self.registers.l);
                8
            }
            0x36 => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.swap(val);
                self.write_byte(hl, result);
                16
            }
            0x37 => {
                self.registers.a = self.swap(self.registers.a);
                8
            }

            // SRL 指令 (0x38-0x3F)
            0x38 => {
                self.registers.b = self.srl(self.registers.b);
                8
            }
            0x39 => {
                self.registers.c = self.srl(self.registers.c);
                8
            }
            0x3A => {
                self.registers.d = self.srl(self.registers.d);
                8
            }
            0x3B => {
                self.registers.e = self.srl(self.registers.e);
                8
            }
            0x3C => {
                self.registers.h = self.srl(self.registers.h);
                8
            }
            0x3D => {
                self.registers.l = self.srl(self.registers.l);
                8
            }
            0x3E => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let val = self.read_byte(hl);
                let result = self.srl(val);
                self.write_byte(hl, result);
                16
            }
            0x3F => {
                self.registers.a = self.srl(self.registers.a);
                8
            }

            // BIT 指令 (0x40-0x7F)
            0x40..=0x7F => {
                let bit = (cb_opcode - 0x40) / 8;

                let reg = (cb_opcode - 0x40) % 8;
                match reg {
                    0 => self.bit_test(bit, self.registers.b),
                    1 => self.bit_test(bit, self.registers.c),
                    2 => self.bit_test(bit, self.registers.d),
                    3 => self.bit_test(bit, self.registers.e),
                    4 => self.bit_test(bit, self.registers.h),
                    5 => self.bit_test(bit, self.registers.l),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let val = self.read_byte(hl);
                        self.bit_test(bit, val);
                        return 12; // (HL) 指令需要額外週期
                    }
                    7 => self.bit_test(bit, self.registers.a),
                    _ => unreachable!(),
                }
                8
            }

            // RES 指令 (0x80-0xBF)
            0x80..=0xBF => {
                let bit = (cb_opcode - 0x80) / 8;
                let reg = (cb_opcode - 0x80) % 8;
                match reg {
                    0 => self.registers.b = self.bit_reset(bit, self.registers.b),
                    1 => self.registers.c = self.bit_reset(bit, self.registers.c),
                    2 => self.registers.d = self.bit_reset(bit, self.registers.d),
                    3 => self.registers.e = self.bit_reset(bit, self.registers.e),
                    4 => self.registers.h = self.bit_reset(bit, self.registers.h),
                    5 => self.registers.l = self.bit_reset(bit, self.registers.l),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let val = self.read_byte(hl);
                        let result = self.bit_reset(bit, val);
                        self.write_byte(hl, result);
                        return 16; // (HL) 指令需要額外週期
                    }
                    7 => self.registers.a = self.bit_reset(bit, self.registers.a),
                    _ => unreachable!(),
                }
                8
            }

            // SET 指令 (0xC0-0xFF)
            0xC0..=0xFF => {
                let bit = (cb_opcode - 0xC0) / 8;
                let reg = (cb_opcode - 0xC0) % 8;
                match reg {
                    0 => self.registers.b = self.bit_set(bit, self.registers.b),
                    1 => self.registers.c = self.bit_set(bit, self.registers.c),
                    2 => self.registers.d = self.bit_set(bit, self.registers.d),
                    3 => self.registers.e = self.bit_set(bit, self.registers.e),
                    4 => self.registers.h = self.bit_set(bit, self.registers.h),
                    5 => self.registers.l = self.bit_set(bit, self.registers.l),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let val = self.read_byte(hl);
                        let result = self.bit_set(bit, val);
                        self.write_byte(hl, result);
                        return 16; // (HL) 指令需要額外週期
                    }
                    7 => self.registers.a = self.bit_set(bit, self.registers.a),
                    _ => unreachable!(),
                }
                8
            }
        }
    }

    // CB 指令的輔助方法

    /// 循環左移
    fn rlc(&mut self, value: u8) -> u8 {
        let carry = (value & 0x80) >> 7;
        let result = (value << 1) | carry;

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(carry != 0);

        result
    }

    /// 循環右移
    fn rrc(&mut self, value: u8) -> u8 {
        let carry = value & 0x01;
        let result = (value >> 1) | (carry << 7);

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(carry != 0);

        result
    }

    /// 通過進位旗標的左移
    fn rl(&mut self, value: u8) -> u8 {
        let old_carry = if self.registers.get_c() { 1 } else { 0 };
        let new_carry = (value & 0x80) >> 7;
        let result = (value << 1) | old_carry;

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(new_carry != 0);

        result
    }

    /// 通過進位旗標的右移
    fn rr(&mut self, value: u8) -> u8 {
        let old_carry = if self.registers.get_c() { 0x80 } else { 0 };
        let new_carry = value & 0x01;
        let result = (value >> 1) | old_carry;

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(new_carry != 0);

        result
    }

    /// 算術左移
    fn sla(&mut self, value: u8) -> u8 {
        let carry = (value & 0x80) >> 7;
        let result = value << 1;

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(carry != 0);

        result
    }

    /// 算術右移
    fn sra(&mut self, value: u8) -> u8 {
        let carry = value & 0x01;
        let result = (value >> 1) | (value & 0x80); // 保留符號位

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(carry != 0);

        result
    }

    /// 邏輯右移
    fn srl(&mut self, value: u8) -> u8 {
        let carry = value & 0x01;
        let result = value >> 1;

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(carry != 0);

        result
    }

    /// 交換高低位
    fn swap(&mut self, value: u8) -> u8 {
        let result = (value >> 4) | (value << 4);

        self.registers.set_z(result == 0);
        self.registers.set_n(false);
        self.registers.set_h(false);
        self.registers.set_c(false);

        result
    }

    /// 位測試
    fn bit_test(&mut self, bit: u8, value: u8) {
        let is_set = (value & (1 << bit)) != 0;

        self.registers.set_z(!is_set);
        self.registers.set_n(false);
        self.registers.set_h(true);
        // C 旗標不變
    }

    /// 位重置
    fn bit_reset(&self, bit: u8, value: u8) -> u8 {
        value & !(1 << bit)
    }

    /// 位設置
    fn bit_set(&self, bit: u8, value: u8) -> u8 {
        value | (1 << bit)
    }

    /// 根據寄存器代碼獲取寄存器值
    fn get_register_value(&self, reg: u8) -> u8 {
        match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.read_byte(hl)
            }
            7 => self.registers.a,
            _ => panic!("無效的寄存器代碼: {}", reg),
        }
    }

    /// 根據寄存器代碼設置寄存器值
    fn set_register_value(&mut self, reg: u8, value: u8) {
        match reg {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => {
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.write_byte(hl, value);
            }
            7 => self.registers.a = value,
            _ => panic!("無效的寄存器代碼: {}", reg),
        }
    }

    /// 將指令反組譯為人類可讀的格式
    pub fn disassemble_instruction(&self, pc: u16, opcode: u8) -> String {
        match opcode {
            0x00 => "NOP".into(),
            0x01 => {
                let nn = self.read_word(pc + 1);
                format!("LD BC,${:04X}", nn)
            }
            0x02 => "LD (BC),A".into(),
            0x03 => "INC BC".into(),
            0x04 => "INC B".into(),
            0x05 => "DEC B".into(),
            0x06 => {
                let n = self.read_byte(pc + 1);
                format!("LD B,${:02X}", n)
            }
            0x07 => "RLCA".into(),
            0x08 => {
                let nn = self.read_word(pc + 1);
                format!("LD (${:04X}),SP", nn)
            }
            0x09 => "ADD HL,BC".into(),
            0x0A => "LD A,(BC)".into(),
            0x0B => "DEC BC".into(),
            0x0C => "INC C".into(),
            0x0D => "DEC C".into(),
            0x0E => {
                let n = self.read_byte(pc + 1);
                format!("LD C,${:02X}", n)
            }
            0x0F => "RRCA".into(),

            0x10 => "STOP".into(),
            0x11 => {
                let nn = self.read_word(pc + 1);
                format!("LD DE,${:04X}", nn)
            }
            0x12 => "LD (DE),A".into(),
            0x13 => "INC DE".into(),
            0x14 => "INC D".into(),
            0x15 => "DEC D".into(),
            0x16 => {
                let n = self.read_byte(pc + 1);
                format!("LD D,${:02X}", n)
            }
            0x17 => "RLA".into(),
            0x18 => {
                let n = self.read_byte(pc + 1) as i8;
                format!("JR ${:04X}", (pc as i16 + 2 + n as i16) as u16)
            }
            0x19 => "ADD HL,DE".into(),
            0x1A => "LD A,(DE)".into(),
            0x1B => "DEC DE".into(),
            0x1C => "INC E".into(),
            0x1D => "DEC E".into(),
            0x1E => {
                let n = self.read_byte(pc + 1);
                format!("LD E,${:02X}", n)
            }
            0x1F => "RRA".into(),

            0x20 => {
                let n = self.read_byte(pc + 1) as i8;
                format!("JR NZ,${:04X}", (pc as i16 + 2 + n as i16) as u16)
            }
            0x21 => {
                let nn = self.read_word(pc + 1);
                format!("LD HL,${:04X}", nn)
            }
            0x22 => "LD (HL+),A".into(),
            0x23 => "INC HL".into(),
            0x24 => "INC H".into(),
            0x25 => "DEC H".into(),
            0x26 => {
                let n = self.read_byte(pc + 1);
                format!("LD H,${:02X}", n)
            }
            0x27 => "DAA".into(),
            0x28 => {
                let n = self.read_byte(pc + 1) as i8;
                format!("JR Z,${:04X}", (pc as i16 + 2 + n as i16) as u16)
            }
            0x29 => "ADD HL,HL".into(),
            0x2A => "LD A,(HL+)".into(),
            0x2B => "DEC HL".into(),
            0x2C => "INC L".into(),
            0x2D => "DEC L".into(),
            0x2E => {
                let n = self.read_byte(pc + 1);
                format!("LD L,${:02X}", n)
            }
            0x2F => "CPL".into(),

            0x30 => {
                let n = self.read_byte(pc + 1) as i8;
                format!("JR NC,${:04X}", (pc as i16 + 2 + n as i16) as u16)
            }
            0x31 => {
                let nn = self.read_word(pc + 1);
                format!("LD SP,${:04X}", nn)
            }
            0x32 => "LD (HL-),A".into(),
            0x33 => "INC SP".into(),
            0x34 => "INC (HL)".into(),
            0x35 => "DEC (HL)".into(),
            0x36 => {
                let n = self.read_byte(pc + 1);
                format!("LD (HL),${:02X}", n)
            }
            0x37 => "SCF".into(),
            0x38 => {
                let n = self.read_byte(pc + 1) as i8;
                format!("JR C,${:04X}", (pc as i16 + 2 + n as i16) as u16)
            }
            0x39 => "ADD HL,SP".into(),
            0x3A => "LD A,(HL-)".into(),
            0x3B => "DEC SP".into(),
            0x3C => "INC A".into(),
            0x3D => "DEC A".into(),
            0x3E => {
                let n = self.read_byte(pc + 1);
                format!("LD A,${:02X}", n)
            }
            0x3F => "CCF".into(),

            // LD r,r' 指令 (0x40-0x7F)
            0x40 => "LD B,B".into(),
            0x41 => "LD B,C".into(),
            0x42 => "LD B,D".into(),
            0x43 => "LD B,E".into(),
            0x44 => "LD B,H".into(),
            0x45 => "LD B,L".into(),
            0x46 => "LD B,(HL)".into(),
            0x47 => "LD B,A".into(),
            0x48 => "LD C,B".into(),
            0x49 => "LD C,C".into(),
            0x4A => "LD C,D".into(),
            0x4B => "LD C,E".into(),
            0x4C => "LD C,H".into(),
            0x4D => "LD C,L".into(),
            0x4E => "LD C,(HL)".into(),
            0x4F => "LD C,A".into(),
            0x50 => "LD D,B".into(),
            0x51 => "LD D,C".into(),
            0x52 => "LD D,D".into(),
            0x53 => "LD D,E".into(),
            0x54 => "LD D,H".into(),
            0x55 => "LD D,L".into(),
            0x56 => "LD D,(HL)".into(),
            0x57 => "LD D,A".into(),
            0x58 => "LD E,B".into(),
            0x59 => "LD E,C".into(),
            0x5A => "LD E,D".into(),
            0x5B => "LD E,E".into(),
            0x5C => "LD E,H".into(),
            0x5D => "LD E,L".into(),
            0x5E => "LD E,(HL)".into(),
            0x5F => "LD E,A".into(),
            0x60 => "LD H,B".into(),
            0x61 => "LD H,C".into(),
            0x62 => "LD H,D".into(),
            0x63 => "LD H,E".into(),
            0x64 => "LD H,H".into(),
            0x65 => "LD H,L".into(),
            0x66 => "LD H,(HL)".into(),
            0x67 => "LD H,A".into(),
            0x68 => "LD L,B".into(),
            0x69 => "LD L,C".into(),
            0x6A => "LD L,D".into(),
            0x6B => "LD L,E".into(),
            0x6C => "LD L,H".into(),
            0x6D => "LD L,L".into(),
            0x6E => "LD L,(HL)".into(),
            0x6F => "LD L,A".into(),
            0x70 => "LD (HL),B".into(),
            0x71 => "LD (HL),C".into(),
            0x72 => "LD (HL),D".into(),
            0x73 => "LD (HL),E".into(),
            0x74 => "LD (HL),H".into(),
            0x75 => "LD (HL),L".into(),
            0x76 => "HALT".into(),
            0x77 => "LD (HL),A".into(),
            0x78 => "LD A,B".into(),
            0x79 => "LD A,C".into(),
            0x7A => "LD A,D".into(),
            0x7B => "LD A,E".into(),
            0x7C => "LD A,H".into(),
            0x7D => "LD A,L".into(),
            0x7E => "LD A,(HL)".into(),
            0x7F => "LD A,A".into(),

            // ADD 指令 (0x80-0x87)
            0x80 => "ADD A,B".into(),
            0x81 => "ADD A,C".into(),
            0x82 => "ADD A,D".into(),
            0x83 => "ADD A,E".into(),
            0x84 => "ADD A,H".into(),
            0x85 => "ADD A,L".into(),
            0x86 => "ADD A,(HL)".into(),
            0x87 => "ADD A,A".into(),

            // ADC 指令 (0x88-0x8F)
            0x88 => "ADC A,B".into(),
            0x89 => "ADC A,C".into(),
            0x8A => "ADC A,D".into(),
            0x8B => "ADC A,E".into(),
            0x8C => "ADC A,H".into(),
            0x8D => "ADC A,L".into(),
            0x8E => "ADC A,(HL)".into(),
            0x8F => "ADC A,A".into(),

            // SUB 指令 (0x90-0x97)
            0x90 => "SUB A,B".into(),
            0x91 => "SUB A,C".into(),
            0x92 => "SUB A,D".into(),
            0x93 => "SUB A,E".into(),
            0x94 => "SUB A,H".into(),
            0x95 => "SUB A,L".into(),
            0x96 => "SUB A,(HL)".into(),
            0x97 => "SUB A,A".into(),

            // SBC 指令 (0x98-0x9F)
            0x98 => "SBC A,B".into(),
            0x99 => "SBC A,C".into(),
            0x9A => "SBC A,D".into(),
            0x9B => "SBC A,E".into(),
            0x9C => "SBC A,H".into(),
            0x9D => "SBC A,L".into(),
            0x9E => "SBC A,(HL)".into(),
            0x9F => "SBC A,A".into(),

            // AND 指令 (0xA0-0xA7)
            0xA0 => "AND A,B".into(),
            0xA1 => "AND A,C".into(),
            0xA2 => "AND A,D".into(),
            0xA3 => "AND A,E".into(),
            0xA4 => "AND A,H".into(),
            0xA5 => "AND A,L".into(),
            0xA6 => "AND A,(HL)".into(),
            0xA7 => "AND A,A".into(),

            // XOR 指令 (0xA8-0xAF)
            0xA8 => "XOR A,B".into(),
            0xA9 => "XOR A,C".into(),
            0xAA => "XOR A,D".into(),
            0xAB => "XOR A,E".into(),
            0xAC => "XOR A,H".into(),
            0xAD => "XOR A,L".into(),
            0xAE => "XOR A,(HL)".into(),
            0xAF => "XOR A,A".into(),

            // OR 指令 (0xB0-0xB7)
            0xB0 => "OR A,B".into(),
            0xB1 => "OR A,C".into(),
            0xB2 => "OR A,D".into(),
            0xB3 => "OR A,E".into(),
            0xB4 => "OR A,H".into(),
            0xB5 => "OR A,L".into(),
            0xB6 => "OR A,(HL)".into(),
            0xB7 => "OR A,A".into(),

            // CP 指令 (0xB8-0xBF)
            0xB8 => "CP A,B".into(),
            0xB9 => "CP A,C".into(),
            0xBA => "CP A,D".into(),
            0xBB => "CP A,E".into(),
            0xBC => "CP A,H".into(),
            0xBD => "CP A,L".into(),
            0xBE => "CP A,(HL)".into(),
            0xBF => "CP A,A".into(),

            // 條件返回指令
            0xC0 => "RET NZ".into(),
            0xC8 => "RET Z".into(),
            0xD0 => "RET NC".into(),
            0xD8 => "RET C".into(),

            // POP 指令
            0xC1 => "POP BC".into(),
            0xD1 => "POP DE".into(),
            0xE1 => "POP HL".into(),
            0xF1 => "POP AF".into(),

            // 條件跳轉指令
            0xC2 => {
                let nn = self.read_word(pc + 1);
                format!("JP NZ,${:04X}", nn)
            }
            0xCA => {
                let nn = self.read_word(pc + 1);
                format!("JP Z,${:04X}", nn)
            }
            0xD2 => {
                let nn = self.read_word(pc + 1);
                format!("JP NC,${:04X}", nn)
            }
            0xDA => {
                let nn = self.read_word(pc + 1);
                format!("JP C,${:04X}", nn)
            }

            // 條件調用指令
            0xC4 => {
                let nn = self.read_word(pc + 1);
                format!("CALL NZ,${:04X}", nn)
            }
            0xCC => {
                let nn = self.read_word(pc + 1);
                format!("CALL Z,${:04X}", nn)
            }
            0xD4 => {
                let nn = self.read_word(pc + 1);
                format!("CALL NC,${:04X}", nn)
            }
            0xDC => {
                let nn = self.read_word(pc + 1);
                format!("CALL C,${:04X}", nn)
            }

            // PUSH 指令
            0xC5 => "PUSH BC".into(),
            0xD5 => "PUSH DE".into(),
            0xE5 => "PUSH HL".into(),
            0xF5 => "PUSH AF".into(),

            // 立即數算術指令
            0xC6 => {
                let n = self.read_byte(pc + 1);
                format!("ADD A,${:02X}", n)
            }
            0xD6 => {
                let n = self.read_byte(pc + 1);
                format!("SUB A,${:02X}", n)
            }
            0xE6 => {
                let n = self.read_byte(pc + 1);
                format!("AND A,${:02X}", n)
            }
            0xF6 => {
                let n = self.read_byte(pc + 1);
                format!("OR A,${:02X}", n)
            }
            0xCE => {
                let n = self.read_byte(pc + 1);
                format!("ADC A,${:02X}", n)
            }
            0xDE => {
                let n = self.read_byte(pc + 1);
                format!("SBC A,${:02X}", n)
            }
            0xEE => {
                let n = self.read_byte(pc + 1);
                format!("XOR A,${:02X}", n)
            }
            0xFE => {
                let n = self.read_byte(pc + 1);
                format!("CP A,${:02X}", n)
            }

            // RST 指令
            0xC7 => "RST 00H".into(),
            0xCF => "RST 08H".into(),
            0xD7 => "RST 10H".into(),
            0xDF => "RST 18H".into(),
            0xE7 => "RST 20H".into(),
            0xEF => "RST 28H".into(),
            0xF7 => "RST 30H".into(),
            0xFF => "RST 38H".into(),

            // 無條件調用
            0xCD => {
                let nn = self.read_word(pc + 1);
                format!("CALL ${:04X}", nn)
            }

            // I/O 指令
            0xE0 => {
                let n = self.read_byte(pc + 1);
                format!("LD (FF00+${:02X}),A", n)
            }
            0xF0 => {
                let n = self.read_byte(pc + 1);
                format!("LD A,(FF00+${:02X})", n)
            }
            0xE2 => "LD (FF00+C),A".into(),
            0xF2 => "LD A,(FF00+C)".into(),

            // 16位記憶體操作
            0xEA => {
                let nn = self.read_word(pc + 1);
                format!("LD (${:04X}),A", nn)
            }
            0xFA => {
                let nn = self.read_word(pc + 1);
                format!("LD A,(${:04X})", nn)
            }

            // 堆疊指標操作
            0xE8 => {
                let n = self.read_byte(pc + 1);
                format!("ADD SP,${:02X}", n)
            }
            0xF8 => {
                let n = self.read_byte(pc + 1);
                format!("LD HL,SP+${:02X}", n)
            }
            0xF9 => "LD SP,HL".into(),

            // 其他重要指令
            0xE9 => "JP (HL)".into(),
            0xD9 => "RETI".into(),
            0xC9 => "RET".into(),
            0xF3 => "DI".into(),
            0xFB => "EI".into(),

            // CB 前綴指令
            0xCB => {
                let cb_opcode = self.read_byte(pc + 1);
                format!("CB {:02X}", cb_opcode)
            }

            // 其他指令
            0xC3 => {
                let nn = self.read_word(pc + 1);
                format!("JP ${:04X}", nn)
            }

            // 未使用/無效的指令
            0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB | 0xEC | 0xED | 0xF4 | 0xFC | 0xFD => {
                format!("INVALID ${:02X}", opcode)
            }
        }
    }
}
