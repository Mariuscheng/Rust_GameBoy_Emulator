// Flag operations

// 暫時注釋掉未使用的常數
// const FLAG_Z: u8 = 0b10000000; // Zero flag
// const FLAG_N: u8 = 0b01000000; // Subtract flag
// const FLAG_H: u8 = 0b00100000; // Half carry flag
// const FLAG_C: u8 = 0b00010000; // Carry flag

pub fn set_flag(flags: &mut u8, flag: u8, value: bool) {
    if value {
        *flags |= flag;
    } else {
        *flags &= !flag;
    }
}

pub fn get_flag(flags: u8, flag: u8) -> bool {
    (flags & flag) != 0
}
