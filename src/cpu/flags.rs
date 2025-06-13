// 標誌位常量
pub const FLAG_Z: u8 = 0b1000_0000; // Zero Flag (Bit 7)
pub const FLAG_N: u8 = 0b0100_0000; // Subtract Flag (Bit 6)
pub const FLAG_H: u8 = 0b0010_0000; // Half Carry Flag (Bit 5)
pub const FLAG_C: u8 = 0b0001_0000; // Carry Flag (Bit 4)
pub const FLAG_UNUSED: u8 = 0b0000_1111; // 低4位未使用，總是為0

use super::Registers;

pub trait FlagOperations {
    fn get_z_flag(&self) -> bool;
    fn get_n_flag(&self) -> bool;
    fn get_h_flag(&self) -> bool;
    fn get_c_flag(&self) -> bool;
    fn set_z_flag(&mut self, value: bool);
    fn set_n_flag(&mut self, value: bool);
    fn set_h_flag(&mut self, value: bool);
    fn set_c_flag(&mut self, value: bool);

    // 新增輔助方法
    fn reset_flags(&mut self);
    fn update_flags_add(&mut self, value: u8, addend: u8, carry: bool);
    fn update_flags_sub(&mut self, value: u8, subtrahend: u8, carry: bool);
    fn update_flags_logic(&mut self, result: u8);
}

impl FlagOperations for Registers {
    fn get_z_flag(&self) -> bool {
        (self.f & FLAG_Z) != 0
    }

    fn get_n_flag(&self) -> bool {
        (self.f & FLAG_N) != 0
    }

    fn get_h_flag(&self) -> bool {
        (self.f & FLAG_H) != 0
    }

    fn get_c_flag(&self) -> bool {
        (self.f & FLAG_C) != 0
    }

    fn reset_flags(&mut self) {
        self.f = 0;
    }    fn update_flags_add(&mut self, value: u8, addend: u8, carry: bool) {
        let carry_value = if carry { 1u8 } else { 0 };
        let result = (value as u16) + (addend as u16) + (carry_value as u16);
        let half_result = ((value & 0x0F) + (addend & 0x0F) + carry_value) as u16;

        self.set_z_flag((result as u8) == 0);
        self.set_n_flag(false);
        self.set_h_flag(half_result > 0x0F);
        self.set_c_flag(result > 0xFF);
    }

    fn update_flags_sub(&mut self, value: u8, subtrahend: u8, carry: bool) {
        let carry_value = if carry { 1 } else { 0 };
        let result = value.wrapping_sub(subtrahend).wrapping_sub(carry_value);
        
        self.set_z_flag(result == 0);
        self.set_n_flag(true);
        self.set_h_flag((value & 0x0F) < (subtrahend & 0x0F) + carry_value);
        self.set_c_flag((value as u16) < (subtrahend as u16) + (carry_value as u16));
    }

    fn update_flags_logic(&mut self, result: u8) {
        self.set_z_flag(result == 0);
        self.set_n_flag(false);
        self.set_h_flag(false);
        self.set_c_flag(false);
    }

    // 確保未使用的位總是為0
    fn set_z_flag(&mut self, value: bool) {
        if value {
            self.f = (self.f | FLAG_Z) & !FLAG_UNUSED;
        } else {
            self.f = (self.f & !FLAG_Z) & !FLAG_UNUSED;
        }
    }

    fn set_n_flag(&mut self, value: bool) {
        if value {
            self.f = (self.f | FLAG_N) & !FLAG_UNUSED;
        } else {
            self.f = (self.f & !FLAG_N) & !FLAG_UNUSED;
        }
    }

    fn set_h_flag(&mut self, value: bool) {
        if value {
            self.f = (self.f | FLAG_H) & !FLAG_UNUSED;
        } else {
            self.f = (self.f & !FLAG_H) & !FLAG_UNUSED;
        }
    }

    fn set_c_flag(&mut self, value: bool) {
        if value {
            self.f = (self.f | FLAG_C) & !FLAG_UNUSED;
        } else {
            self.f = (self.f & !FLAG_C) & !FLAG_UNUSED;
        }
    }
}
