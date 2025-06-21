use super::flags::*;
use super::instructions::register_utils::FlagOperations;
use crate::error::{RegTarget, Result};

#[derive(Debug, Default)]
pub struct Registers {
    pub a: u8,    // 累加器 A
    flags: Flags, // 標誌寄存器 F
    pub b: u8,    // B 寄存器
    pub c: u8,    // C 寄存器
    pub d: u8,    // D 寄存器
    pub e: u8,    // E 寄存器
    pub h: u8,    // H 寄存器
    pub l: u8,    // L 寄存器
    pub sp: u16,  // 堆疊指針
    pub pc: u16,  // 程式計數器
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0x01,
            flags: Flags::new(0xB0),
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
        }
    } // 基本寄存器操作
    pub fn get_register(&self, reg: RegTarget) -> Result<u8> {
        match reg {
            RegTarget::A => Ok(self.a),
            RegTarget::B => Ok(self.b),
            RegTarget::C => Ok(self.c),
            RegTarget::D => Ok(self.d),
            RegTarget::E => Ok(self.e),
            RegTarget::H => Ok(self.h),
            RegTarget::L => Ok(self.l),
            RegTarget::HL => {
                // 對於 HL 作為記憶體位置，這個應該在 CPU 層面處理
                // 這裡只返回 HL 位址本身
                Ok(((self.h as u16) << 8 | self.l as u16) as u8)
            }
            _ => Err(crate::error::Error::Instruction(
                crate::error::InstructionError::InvalidRegister(reg),
            )),
        }
    }

    pub fn set_register(&mut self, reg: RegTarget, value: u8) -> Result<()> {
        match reg {
            RegTarget::A => {
                self.a = value;
                Ok(())
            }
            RegTarget::B => {
                self.b = value;
                Ok(())
            }
            RegTarget::C => {
                self.c = value;
                Ok(())
            }
            RegTarget::D => {
                self.d = value;
                Ok(())
            }
            RegTarget::E => {
                self.e = value;
                Ok(())
            }
            RegTarget::H => {
                self.h = value;
                Ok(())
            }
            RegTarget::L => {
                self.l = value;
                Ok(())
            }
            RegTarget::HL => {
                // 對於 HL 作為記憶體位置，這個應該在 CPU 層面處理
                Err(crate::error::Error::Instruction(
                    crate::error::InstructionError::InvalidRegister(reg),
                ))
            }
            _ => Err(crate::error::Error::Instruction(
                crate::error::InstructionError::InvalidRegister(reg),
            )),
        }
    }

    // 保留舊的方法以向後兼容
    pub fn get_reg(&self, reg: RegTarget) -> u8 {
        self.get_register(reg).unwrap_or(0)
    }

    pub fn set_reg(&mut self, reg: RegTarget, value: u8) {
        let _ = self.set_register(reg, value);
    } // Flag operations
    pub fn get_flag(&self, flag: Flag) -> bool {
        self.flags.get(flag as u8)
    }

    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        self.flags.set(flag as u8, value);
    }

    // 16位元寄存器操作
    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | (self.flags.value() as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.flags.set_value(value as u8); // 低4位始終為0
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

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }

    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    pub fn set_sp(&mut self, value: u16) {
        self.sp = value;
    }

    // 8位寄存器的 getter/setter
    pub fn get_a(&self) -> u8 {
        self.a
    }

    pub fn set_a(&mut self, value: u8) {
        self.a = value;
    }

    pub fn get_b(&self) -> u8 {
        self.b
    }

    pub fn set_b(&mut self, value: u8) {
        self.b = value;
    }

    pub fn get_c(&self) -> u8 {
        self.c
    }

    pub fn set_c(&mut self, value: u8) {
        self.c = value;
    }

    pub fn get_d(&self) -> u8 {
        self.d
    }

    pub fn set_d(&mut self, value: u8) {
        self.d = value;
    }

    pub fn get_e(&self) -> u8 {
        self.e
    }

    pub fn set_e(&mut self, value: u8) {
        self.e = value;
    }

    pub fn get_h(&self) -> u8 {
        self.h
    }

    pub fn set_h(&mut self, value: u8) {
        self.h = value;
    }

    pub fn get_l(&self) -> u8 {
        self.l
    }

    pub fn set_l(&mut self, value: u8) {
        self.l = value;
    } // 標誌操作    // 已棄用的標誌位操作方法,使用 trait FlagOperations 的實作替代
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn get_flag_z(&self) -> bool {
        self.get_zero()
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn set_flag_z(&mut self, value: bool) {
        self.set_zero(value);
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn get_flag_n(&self) -> bool {
        self.get_subtract()
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn set_flag_n(&mut self, value: bool) {
        self.set_subtract(value);
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn get_flag_h(&self) -> bool {
        self.get_half_carry()
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn set_flag_h(&mut self, value: bool) {
        self.set_half_carry(value);
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn get_flag_c(&self) -> bool {
        self.get_carry()
    }
    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn set_flag_c(&mut self, value: bool) {
        self.set_carry(value);
    }

    // 組合標誌操作
    pub fn update_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.flags.set_value(0);
        self.flags.set_zero(z);
        self.flags.set_subtract(n);
        self.flags.set_half_carry(h);
        self.flags.set_carry(c);
    }
    pub fn update_zero_and_carry(&mut self, z: bool, c: bool) {
        let n = self.get_subtract();
        let h = self.get_half_carry();
        self.update_flags(z, n, h, c);
    }

    // 取得標誌寄存器的值
    pub fn get_f(&self) -> u8 {
        self.flags.value()
    }

    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn set_n(&mut self, value: bool) {
        self.set_subtract(value);
    }

    #[deprecated(
        since = "2025-06-21",
        note = "please use trait FlagOperations methods instead"
    )]
    pub fn get_n(&self) -> bool {
        self.get_subtract()
    }
}

/// CPU 標誌位處理 trait
impl FlagOperations for Registers {
    fn set_zero(&mut self, value: bool) {
        self.set_flag(Flag::Z, value);
    }

    fn set_subtract(&mut self, value: bool) {
        self.set_flag(Flag::N, value);
    }

    fn set_half_carry(&mut self, value: bool) {
        self.set_flag(Flag::H, value);
    }

    fn set_carry(&mut self, value: bool) {
        self.set_flag(Flag::C, value);
    }

    fn get_zero(&self) -> bool {
        self.get_flag(Flag::Z)
    }

    fn get_subtract(&self) -> bool {
        self.get_flag(Flag::N)
    }

    fn get_half_carry(&self) -> bool {
        self.get_flag(Flag::H)
    }

    fn get_carry(&self) -> bool {
        self.get_flag(Flag::C)
    }
}
