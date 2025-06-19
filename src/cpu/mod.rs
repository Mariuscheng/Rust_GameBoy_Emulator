use crate::cpu::instructions::common::{CYCLES_1, CYCLES_4};
use crate::error::Result;
use crate::mmu::MMU;
use std::{cell::RefCell, io::Write, rc::Rc};

pub mod decode_and_execute;
pub mod flags;
pub mod instructions;
pub mod interrupts;
pub mod registers;

use crate::cpu::interrupts::InterruptRegisters;
use registers::Registers;

#[derive(Debug, PartialEq)]
pub enum CpuState {
    Running,
    Halted,
    Stopped,
}

pub struct CPU {
    pub registers: Registers,
    pub mmu: Rc<RefCell<MMU>>,
    pub interrupt_registers: Rc<RefCell<InterruptRegisters>>,
    instruction_count: u64,
    ime: bool,                        // 中斷主開關
    ei_pending: bool,                 // EI 指令等待執行
    state: CpuState,                  // CPU 狀態
    total_cycles: u64,                // 總執行週期數
    loop_detection: CpuLoopDetection, // 循環偵測
}

#[derive(Default, Debug)]
struct CpuLoopDetection {
    pub last_pc: u16,
    pub repeat_count: u32,
    pub loop_detected: bool,
    pub detection_enabled: bool,
}

impl CpuLoopDetection {
    fn new(enabled: bool) -> Self {
        Self {
            last_pc: 0,
            repeat_count: 0,
            loop_detected: false,
            detection_enabled: enabled,
        }
    }

    fn check(&mut self, pc: u16) -> bool {
        if !self.detection_enabled {
            return false;
        }

        if pc == self.last_pc {
            self.repeat_count += 1;
            if self.repeat_count > 1000 {
                self.loop_detected = true;
                return true;
            }
        } else {
            self.last_pc = pc;
            self.repeat_count = 0;
            self.loop_detected = false;
        }
        false
    }

    fn reset(&mut self) {
        self.repeat_count = 0;
        self.loop_detected = false;
    }
}

impl CPU {
    pub fn new(
        mmu: Rc<RefCell<MMU>>,
        interrupt_registers: Rc<RefCell<InterruptRegisters>>,
    ) -> Self {
        Self {
            registers: Registers::new(),
            mmu,
            interrupt_registers,
            instruction_count: 0,
            ime: false,
            ei_pending: false,
            state: CpuState::Running,
            total_cycles: 0,
            loop_detection: CpuLoopDetection::new(true),
        }
    }

    pub fn fetch_byte(&mut self) -> Result<u8> {
        let byte = self.read_byte(self.registers.pc)?;
        self.registers.pc = self.registers.pc.wrapping_add(1);
        Ok(byte)
    }

    fn read_byte(&self, addr: u16) -> Result<u8> {
        match self.mmu.borrow().read_byte(addr) {
            Ok(value) => {
                // log: 讀取位址 {:#06X}: {:#04X}
                Ok(value)
            }
            Err(e) => {
                // log: 讀取位址 {:#06X} 失敗: {}
                Err(e)
            }
        }
    }

    fn write_byte(&self, addr: u16, value: u8) -> Result<()> {
        match self.mmu.borrow_mut().write_byte(addr, value) {
            Ok(_) => {
                // log: 寫入位址 {:#06X}: {:#04X}
                Ok(())
            }
            Err(e) => {
                // log: 寫入位址 {:#06X} 失敗: {}
                Err(e)
            }
        }
    }

    fn read_word(&self, addr: u16) -> Result<u16> {
        let low = self.read_byte(addr)?;
        let high = self.read_byte(addr.wrapping_add(1))?;
        Ok(u16::from_le_bytes([low, high]))
    }

    fn write_word(&self, addr: u16, value: u16) -> Result<()> {
        let [low, high] = value.to_le_bytes();
        self.write_byte(addr, low)?;
        self.write_byte(addr.wrapping_add(1), high)
    }

    fn fetch_word(&mut self) -> Result<u16> {
        let value = self.read_word(self.registers.pc)?;
        self.registers.pc = self.registers.pc.wrapping_add(2);
        Ok(value)
    }

    #[allow(dead_code)]
    fn push_word(&mut self, value: u16) -> Result<()> {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.write_word(self.registers.sp, value)
    }

    #[allow(dead_code)]
    fn pop_word(&mut self) -> Result<u16> {
        let value = self.read_word(self.registers.sp)?;
        self.registers.sp = self.registers.sp.wrapping_add(2);
        Ok(value)
    }

    /// 獲取總執行週期數
    pub fn get_total_cycles(&self) -> u64 {
        self.total_cycles
    }

    /// 檢查 CPU 是否處於停止狀態
    pub fn is_stopped(&self) -> bool {
        self.state == CpuState::Stopped
    }
    /// 處理中斷
    fn handle_interrupts(&mut self) -> Result<bool> {
        // 如果 IME 未啟用且 CPU 不是處於暫停狀態，不處理中斷
        if !self.ime && self.state != CpuState::Halted {
            return Ok(false);
        }

        // 檢查中斷狀態
        let pending = {
            let interrupt_regs = self.interrupt_registers.borrow();
            interrupt_regs.get_enabled() & interrupt_regs.get_flags()
        };

        // 沒有待處理的中斷
        if pending == 0 {
            return Ok(false);
        }

        // 如果 CPU 處於暫停狀態，恢復運行
        if self.state == CpuState::Halted {
            self.state = CpuState::Running;
            if !self.ime {
                return Ok(false);
            }
        }

        // 禁用中斷
        self.ime = false;

        // 處理優先級最高的中斷
        for i in 0..5 {
            let mask = 1 << i;
            if pending & mask != 0 {
                let interrupt_vector = 0x40 + (i << 3);
                self.push_pc_for_interrupt()?;
                self.registers.pc = interrupt_vector;
                self.interrupt_registers
                    .borrow_mut()
                    .set_interrupt_flag(mask, false);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 執行一條指令
    pub fn step(&mut self) -> Result<u8> {
        // 檢測可能的循環
        if self.loop_detection.check(self.registers.pc) {
            // log: 檢測到可能的死循環在 PC=0x{:04X}, 已執行 {} 次
            if self.registers.pc >= 0x0200
                && self.registers.pc <= 0x0300
                && self.loop_detection.repeat_count > 1000
            {
                println!(
                    "[警告] ROM 初始化循環偵測：PC=0x{:04X} 已重複執行超過1000次，自動跳到 0x0100",
                    self.registers.pc
                );
                self.registers.pc = 0x0100;
                self.loop_detection.reset();
            }
        }

        // 處理中斷
        if self.handle_interrupts()? {
            self.instruction_count += 1;
            return Ok(CYCLES_4);
        }

        // 如果處於停止狀態，返回
        if self.state == CpuState::Stopped {
            return Ok(CYCLES_1);
        }

        // 如果處於暫停狀態且沒有中斷，返回
        if self.state == CpuState::Halted {
            return Ok(CYCLES_1);
        }

        // 更新 EI 指令的延遲效果
        if self.ei_pending {
            self.ime = true;
            self.ei_pending = false;
        }

        // 讀取並執行指令
        let opcode = self.read_next_byte()?;
        let cycles = instructions::dispatch(self, opcode)?;
        self.total_cycles += cycles as u64;
        self.instruction_count += 1;

        // 檢查循環
        if self.loop_detection.check(self.registers.pc) {
            // log: 偵測到潛在的 CPU 循環: PC = {:#06X}
        }

        // 強化死循環自動跳出
        if self.registers.pc >= 0x0200 && self.registers.pc <= 0x0300 {
            self.loop_detection.repeat_count += 1;
            if self.loop_detection.repeat_count > 1000 {
                use std::fs::OpenOptions;
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                    let _ = writeln!(file, "[警告] ROM 初始化死循環自動跳出：PC=0x{:04X}，強制跳到 0x0100", self.registers.pc);
                }
                self.registers.pc = 0x0100;
                self.loop_detection.repeat_count = 0;
            }
        } else {
            self.loop_detection.repeat_count = 0;
        }

        // DEBUG: 每步記錄 PC, opcode, HL, A
        let pc = self.registers.pc;
        let opcode = self.mmu.borrow().read_byte(pc).unwrap_or(0xFF);
        let hl = self.registers.get_hl();
        let a = self.registers.a;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
            let _ = writeln!(file, "[CPU] PC={:04X} OPCODE={:02X} HL={:04X} A={:02X}", pc, opcode, hl, a);
        }

        // DEBUG: 進入中斷服務程式時 log
        if self.registers.pc == 0x0040 {
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                let _ = writeln!(file, "[CPU-ISR] Enter VBlank ISR (PC=0040)");
            }
        }
        if self.registers.pc == 0x0048 {
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                let _ = writeln!(file, "[CPU-ISR] Enter LCD STAT ISR (PC=0048)");
            }
        }

        Ok(cycles)
    }
    /// Push value onto stack
    fn push(&mut self, value: u16) -> Result<()> {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.write_word(self.registers.sp, value)
    }

    fn push_pc_for_interrupt(&mut self) -> Result<()> {
        let pc = self.registers.pc;
        let sp = self.registers.sp.wrapping_sub(2);
        self.registers.sp = sp;

        let mut mmu = self.mmu.borrow_mut();
        mmu.write_word(sp, pc)?;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.registers.pc = 0x0100; // Game Boy ROM 的標準入口點
        self.registers.sp = 0xFFFE; // 堆疊指針的初始值
        self.ime = false; // 禁用中斷
        self.state = CpuState::Running;
        // log: CPU 已重置：PC=0x0100, SP=0xFFFE
    }

    // 獲取當前程序計數器的值
    pub fn get_pc(&self) -> u16 {
        self.registers.pc
    }

    // 設置程序計數器的值
    pub fn set_pc(&mut self, value: u16) {
        self.registers.pc = value;
    } // 讀取當前指令的操作碼
    pub fn read_current_opcode(&self) -> Result<u8> {
        let pc = self.registers.pc;
        self.mmu.borrow().read_byte(pc).map_err(Into::into)
    }

    // 跳過當前指令
    pub fn skip_current_instruction(&mut self) {
        self.registers.pc = self.registers.pc.wrapping_add(1);
    }

    #[allow(dead_code)]
    fn dump_state(&self) -> String {
        let mut next_bytes = [0u8; 2];
        for i in 0..2 {
            let addr = self.registers.pc.wrapping_add(i as u16);
            next_bytes[i] = match self.read_byte(addr) {
                Ok(value) => value,
                Err(_) => 0xFF,
            };
        }

        format!(
            "A=0x{:02X} F=0x{:02X} BC=0x{:04X} DE=0x{:04X} HL=0x{:04X} SP=0x{:04X} PC=0x{:04X}\n\
             次要指令: [{:02X} {:02X}]",
            self.registers.a,
            self.registers.f,
            self.registers.get_bc(),
            self.registers.get_de(),
            self.registers.get_hl(),
            self.registers.sp,
            self.registers.pc,
            next_bytes[0],
            next_bytes[1]
        )
    }

    #[allow(dead_code)]
    fn fetch_next_bytes(&mut self, count: u16) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(count as usize);
        for i in 0..count {
            let addr = self.registers.pc.wrapping_add(i);
            bytes.push(self.read_byte(addr)?);
        }
        Ok(bytes)
    }

    /// 獲取 HL 寄存器對的值
    pub fn get_hl(&self) -> u16 {
        u16::from_be_bytes([self.registers.h, self.registers.l])
    }

    /// 設置 HL 寄存器對的值
    pub fn set_hl(&mut self, value: u16) {
        let [h, l] = value.to_be_bytes();
        self.registers.h = h;
        self.registers.l = l;
    }

    /// 獲取 BC 寄存器對的值
    pub fn get_bc(&self) -> u16 {
        u16::from_be_bytes([self.registers.b, self.registers.c])
    }

    /// 設置 BC 寄存器對的值
    pub fn set_bc(&mut self, value: u16) {
        let [b, c] = value.to_be_bytes();
        self.registers.b = b;
        self.registers.c = c;
    }

    /// 獲取 DE 寄存器對的值
    pub fn get_de(&self) -> u16 {
        u16::from_be_bytes([self.registers.d, self.registers.e])
    }

    /// 設置 DE 寄存器對的值
    pub fn set_de(&mut self, value: u16) {
        let [d, e] = value.to_be_bytes();
        self.registers.d = d;
        self.registers.e = e;
    }

    /// 獲取 AF 寄存器對的值
    pub fn get_af(&self) -> u16 {
        u16::from_be_bytes([self.registers.a, self.registers.f])
    }

    /// 設置 AF 寄存器對的值
    pub fn set_af(&mut self, value: u16) {
        let [a, f] = value.to_be_bytes();
        self.registers.a = a;
        self.registers.f = f & 0xF0; // 只保留高4位
    }

    pub fn read_next_byte(&mut self) -> Result<u8> {
        let byte = self.mmu.borrow().read_byte(self.registers.pc)?;
        self.registers.pc = self.registers.pc.wrapping_add(1);
        Ok(byte)
    }

    // 啟用中斷
    pub fn enable_interrupts(&mut self) {
        self.ei_pending = true;
    }

    // 禁用中斷
    pub fn disable_interrupts(&mut self) {
        self.ime = false;
    }

    pub fn is_interrupts_enabled(&self) -> bool {
        self.ime
    }
    #[cfg(test)]
    fn check_joypad_interrupt(&self) -> Result<bool> {
        let int_flags = self.interrupt_registers.borrow().if_reg;
        Ok((int_flags & JOYPAD_BIT) != 0)
    }
    pub fn run_bootloader(&mut self) -> Result<()> {
        // 模擬啟動程序的關鍵步驟
        for _ in 0..100 {
            self.step()?;
        }

        // log: 啟動程序執行完成
        // log: PC: 0x{:04X}
        // log: SP: 0x{:04X}
        Ok(())
    }
}
