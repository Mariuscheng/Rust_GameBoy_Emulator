use crate::error::{Error, Result};
use crate::mmu::MMU;
use crate::utils::Logger;
use std::{cell::RefCell, rc::Rc};

pub mod instructions;
pub mod interrupts;
mod registers;

pub use instructions::common::{InstructionError, RegPair};
pub use interrupts::{
    Interrupt, InterruptRegisters, JOYPAD_BIT, LCDC_BIT, SERIAL_BIT, TIMER_BIT, VBLANK_BIT,
};
use registers::Registers;

#[derive(Debug)]
pub struct CPU {
    pub registers: Registers,
    mmu: Rc<RefCell<MMU>>,
    interrupt_registers: Rc<RefCell<InterruptRegisters>>,
    state: CpuState,
    ime: bool,
    #[allow(dead_code)] // 新增 allow(dead_code)
    halted: bool,
    logger: RefCell<Logger>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CpuState {
    Running,
    Halted,
    Stopped,
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
            state: CpuState::Running,
            ime: false,
            halted: false,
            logger: RefCell::new(Logger::new()),
        }
    }

    pub fn fetch_byte(&mut self) -> Result<u8> {
        let byte = self.read_byte(self.registers.pc)?;
        self.registers.pc = self.registers.pc.wrapping_add(1);
        Ok(byte)
    }

    fn read_byte(&self, addr: u16) -> Result<u8> {
        self.mmu.borrow().read_byte(addr)
    }

    fn write_byte(&self, addr: u16, value: u8) -> Result<()> {
        self.mmu.borrow_mut().write_byte(addr, value)
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

    #[allow(dead_code)] // 新增 allow(dead_code)
    fn push_word(&mut self, value: u16) -> Result<()> {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.write_word(self.registers.sp, value)
    }

    #[allow(dead_code)] // 新增 allow(dead_code)
    fn pop_word(&mut self) -> Result<u16> {
        let value = self.read_word(self.registers.sp)?;
        self.registers.sp = self.registers.sp.wrapping_add(2);
        Ok(value)
    }

    pub fn step(&mut self) -> Result<u8> {
        match self.state {
            CpuState::Halted => {
                if self.check_interrupts()? {
                    self.state = CpuState::Running;
                } else {
                    return Ok(4); // 暫停狀態
                }
            }
            CpuState::Stopped => {
                return Ok(4); // 停止狀態
            }
            CpuState::Running => {}
        }

        let cycles = self.execute_instruction()?;
        self.handle_interrupts()?;
        Ok(cycles)
    }
    fn execute_instruction(&mut self) -> Result<u8> {
        let pc = self.registers.pc;
        let opcode = self.read_byte(pc)?;

        // 預讀下一個位元組用於錯誤記錄
        let next_byte = self.read_byte(pc.wrapping_add(1)).unwrap_or(0xFF);
        let next_next_byte = self.read_byte(pc.wrapping_add(2)).unwrap_or(0xFF);

        self.registers.pc = pc.wrapping_add(1);

        let result = instructions::dispatch(self, opcode);

        match result {
            Ok(cycles) => {
                self.logger.borrow_mut().debug(&format!(
                    "指令執行成功：PC=0x{:04X}, 操作碼=0x{:02X}, 參數=[0x{:02X}, 0x{:02X}], 週期={}",
                    pc, opcode, next_byte, next_next_byte, cycles
                ));
                Ok(cycles)
            }
            Err(e) => {
                self.logger.borrow_mut().error(&format!(
                    "指令執行失敗：PC=0x{:04X}, 操作碼=0x{:02X}, 參數=[0x{:02X}, 0x{:02X}], 錯誤={:?}\n\
                     寄存器狀態：A=0x{:02X}, F=0x{:02X}, BC=0x{:04X}, DE=0x{:04X}, HL=0x{:04X}, SP=0x{:04X}",
                    pc, opcode, next_byte, next_next_byte,
                    e,
                    self.registers.a, self.registers.f,
                    self.registers.get_bc(), self.registers.get_de(),
                    self.registers.get_hl(), self.registers.sp
                ));
                Err(Error::InstructionError(e))
            }
        }
    }
    /// 檢查使能的中斷
    #[allow(dead_code)] // 新增 allow(dead_code)
    fn check_enabled_interrupts(&self) -> Result<u8> {
        let interrupts = self.interrupt_registers.borrow();
        Ok(interrupts.ie & interrupts.if_reg)
    }

    /// 檢查中斷
    fn check_interrupts(&self) -> Result<bool> {
        let interrupts = self.interrupt_registers.borrow();
        Ok(interrupts.if_reg != 0)
    }

    /// 處理中斷
    fn handle_interrupts(&mut self) -> Result<bool> {
        // 檢查是否有中斷需要處理
        if !self.ime {
            return Ok(false);
        }

        // 檢查中斷
        let enabled;
        {
            let interrupts = self.interrupt_registers.borrow();
            enabled = interrupts.ie & interrupts.if_reg;
        }
        if enabled == 0 {
            return Ok(false);
        }

        // 依序檢查各個中斷位元
        for i in 0..5 {
            let bit = 1 << i;
            if enabled & bit != 0 {
                // 清除中斷標誌
                {
                    let mut interrupts = self.interrupt_registers.borrow_mut();
                    interrupts.if_reg &= !bit;
                }
                self.ime = false;

                // 保存 PC 到堆疊
                self.push(self.registers.pc)?;

                // 跳轉到中斷向量
                self.registers.pc = 0x0040 + (i as u16) * 0x08;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Push value onto stack
    fn push(&mut self, value: u16) -> Result<()> {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.write_word(self.registers.sp, value)
    }

    pub fn reset(&mut self) {
        self.registers.pc = 0x0100; // Game Boy ROM 的標準入口點
        self.registers.sp = 0xFFFE; // 堆疊指針的初始值
        self.ime = false; // 禁用中斷
        self.state = CpuState::Running;
        self.logger
            .borrow_mut()
            .debug("CPU 已重置：PC=0x0100, SP=0xFFFE");
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

    pub fn enable_interrupts(&mut self) {
        self.ime = true;
    }

    pub fn disable_interrupts(&mut self) {
        self.ime = false;
    }

    pub fn is_interrupts_enabled(&self) -> bool {
        self.ime
    }
}
