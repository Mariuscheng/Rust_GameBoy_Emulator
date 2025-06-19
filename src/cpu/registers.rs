use super::instructions::common::FlagOperations;

// CPU 標誌位定義
pub const ZERO_FLAG: u8 = 1 << 7; // Zero Flag (Z)
pub const SUBTRACT_FLAG: u8 = 1 << 6; // Subtract Flag (N)
pub const HALF_CARRY_FLAG: u8 = 1 << 5; // Half Carry Flag (H)
pub const CARRY_FLAG: u8 = 1 << 4; // Carry Flag (C)

#[derive(Debug, Default)]
pub struct Registers {
    pub a: u8,   // 累加器 A
    pub f: u8,   // 標誌寄存器 F
    pub b: u8,   // B 寄存器
    pub c: u8,   // C 寄存器
    pub d: u8,   // D 寄存器
    pub e: u8,   // E 寄存器
    pub h: u8,   // H 寄存器
    pub l: u8,   // L 寄存器
    pub sp: u16, // 堆疊指針
    pub pc: u16, // 程式計數器
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
        }
    }

    pub fn update_flags(
        &mut self,
        zero: Option<bool>,
        subtract: Option<bool>,
        half_carry: Option<bool>,
        carry: Option<bool>,
    ) {
        if let Some(z) = zero {
            if z {
                self.f |= ZERO_FLAG;
            } else {
                self.f &= !ZERO_FLAG;
            }
        }
        if let Some(n) = subtract {
            if n {
                self.f |= SUBTRACT_FLAG;
            } else {
                self.f &= !SUBTRACT_FLAG;
            }
        }
        if let Some(h) = half_carry {
            if h {
                self.f |= HALF_CARRY_FLAG;
            } else {
                self.f &= !HALF_CARRY_FLAG;
            }
        }
        if let Some(c) = carry {
            if c {
                self.f |= CARRY_FLAG;
            } else {
                self.f &= !CARRY_FLAG;
            }
        }
    }

    // 取得 AF 組合
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    // 取得 BC 組合
    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    // 取得 DE 組合
    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    // 取得 HL 組合
    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    // 設定 AF 組合
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xF0) as u8; // 只保留高 4 位
    }

    // 設定 BC 組合
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    // 設定 DE 組合
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    // 設定 HL 組合
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    // 設置單個標誌位
    pub fn set_zero_flag(&mut self, value: bool) {
        if value {
            self.f |= ZERO_FLAG;
        } else {
            self.f &= !ZERO_FLAG;
        }
    }

    pub fn set_subtract_flag(&mut self, value: bool) {
        if value {
            self.f |= SUBTRACT_FLAG;
        } else {
            self.f &= !SUBTRACT_FLAG;
        }
    }

    pub fn set_half_carry_flag(&mut self, value: bool) {
        if value {
            self.f |= HALF_CARRY_FLAG;
        } else {
            self.f &= !HALF_CARRY_FLAG;
        }
    }

    pub fn set_carry_flag(&mut self, value: bool) {
        if value {
            self.f |= CARRY_FLAG;
        } else {
            self.f &= !CARRY_FLAG;
        }
    }

    // 設置零標誌
    pub fn set_flag_z(&mut self, value: bool) {
        if value {
            self.f |= ZERO_FLAG;
        } else {
            self.f &= !ZERO_FLAG;
        }
    }

    // 設置減法標誌
    pub fn set_flag_n(&mut self, value: bool) {
        if value {
            self.f |= SUBTRACT_FLAG;
        } else {
            self.f &= !SUBTRACT_FLAG;
        }
    }

    // 設置半進位標誌
    pub fn set_flag_h(&mut self, value: bool) {
        if value {
            self.f |= HALF_CARRY_FLAG;
        } else {
            self.f &= !HALF_CARRY_FLAG;
        }
    }

    // 設置進位標誌
    pub fn set_flag_c(&mut self, value: bool) {
        if value {
            self.f |= CARRY_FLAG;
        } else {
            self.f &= !CARRY_FLAG;
        }
    }

    // 獲取零標誌
    pub fn get_flag_z(&self) -> bool {
        self.f & ZERO_FLAG != 0
    }

    // 獲取減法標誌
    pub fn get_flag_n(&self) -> bool {
        self.f & SUBTRACT_FLAG != 0
    }

    // 獲取半進位標誌
    pub fn get_flag_h(&self) -> bool {
        self.f & HALF_CARRY_FLAG != 0
    }

    // 獲取進位標誌
    pub fn get_flag_c(&self) -> bool {
        self.f & CARRY_FLAG != 0
    }

    // 獲取全部標誌位
    pub fn get_flags(&self) -> u8 {
        self.f & 0xF0
    }

    // 設置全部標誌位
    pub fn set_flags(&mut self, value: u8) {
        self.f = (self.f & 0x0F) | (value & 0xF0);
    }

    // 獲取堆疊指針的值
    pub fn get_sp(&self) -> u16 {
        self.sp
    }
}

/// CPU 標誌位處理 trait
impl FlagOperations for Registers {
    fn get_zero_flag(&self) -> bool {
        (self.f & ZERO_FLAG) != 0
    }

    fn set_zero_flag(&mut self, value: bool) {
        if value {
            self.f |= ZERO_FLAG;
        } else {
            self.f &= !ZERO_FLAG;
        }
    }

    fn get_subtract_flag(&self) -> bool {
        (self.f & SUBTRACT_FLAG) != 0
    }

    fn set_subtract_flag(&mut self, value: bool) {
        if value {
            self.f |= SUBTRACT_FLAG;
        } else {
            self.f &= !SUBTRACT_FLAG;
        }
    }

    fn get_half_carry_flag(&self) -> bool {
        (self.f & HALF_CARRY_FLAG) != 0
    }

    fn set_half_carry_flag(&mut self, value: bool) {
        if value {
            self.f |= HALF_CARRY_FLAG;
        } else {
            self.f &= !HALF_CARRY_FLAG;
        }
    }

    fn get_carry_flag(&self) -> bool {
        (self.f & CARRY_FLAG) != 0
    }

    fn set_carry_flag(&mut self, value: bool) {
        if value {
            self.f |= CARRY_FLAG;
        } else {
            self.f &= !CARRY_FLAG;
        }
    }

    fn update_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.set_zero_flag(z);
        self.set_subtract_flag(n);
        self.set_half_carry_flag(h);
        self.set_carry_flag(c);
    }
}
