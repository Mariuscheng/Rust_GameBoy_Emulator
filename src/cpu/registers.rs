#[derive(Clone, Debug, Default)]
pub struct Registers {
    pub a: u8,   // 累加器
    pub b: u8,   // B 寄存器
    pub c: u8,   // C 寄存器
    pub d: u8,   // D 寄存器
    pub e: u8,   // E 寄存器
    pub f: u8,   // 標誌寄存器
    pub h: u8,   // H 寄存器
    pub l: u8,   // L 寄存器
    pub sp: u16, // 堆疊指針
    pub pc: u16, // 程序計數器
}

impl Registers {
    pub fn new() -> Self {
        Self {
            // Game Boy 開機時的默認值
            a: 0x01,
            f: 0xB0, // Z和C標誌位設置
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE, // 堆疊指針初始值
            pc: 0x0100, // 程序計數器從 0x100 開始
        }
    }

    // 16位寄存器對訪問方法
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0xFF) as u8;
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    // 標誌位常量
    const FLAG_Z: u8 = 0b1000_0000;
    const FLAG_N: u8 = 0b0100_0000;
    const FLAG_H: u8 = 0b0010_0000;
    const FLAG_C: u8 = 0b0001_0000;

    pub fn get_z_flag(&self) -> bool {
        self.f & Self::FLAG_Z != 0
    }

    pub fn get_n_flag(&self) -> bool {
        self.f & Self::FLAG_N != 0
    }

    pub fn get_h_flag(&self) -> bool {
        self.f & Self::FLAG_H != 0
    }

    pub fn get_c_flag(&self) -> bool {
        self.f & Self::FLAG_C != 0
    }

    pub fn set_z_flag(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_Z;
        } else {
            self.f &= !Self::FLAG_Z;
        }
    }

    pub fn set_n_flag(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_N;
        } else {
            self.f &= !Self::FLAG_N;
        }
    }

    pub fn set_h_flag(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_H;
        } else {
            self.f &= !Self::FLAG_H;
        }
    }

    pub fn set_c_flag(&mut self, value: bool) {
        if value {
            self.f |= Self::FLAG_C;
        } else {
            self.f &= !Self::FLAG_C;
        }
    }
}
