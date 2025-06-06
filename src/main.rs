use minifb::{Key, Window, WindowOptions};
use std::fs;

mod cpu {
    use super::mmu::MMU;

    

    #[derive(Default)]
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

    pub struct CPU {
        registers: Registers,
        mmu: MMU,
        ime: bool,
        timer_counter: u32,
        divider_counter: u32,
        pub scanline_counter: u32,
    }

    impl CPU {
        pub fn new(mmu: MMU) -> Self {
            CPU {
                registers: Registers {
                    pc: 0x0100,
                    sp: 0xFFFE,
                    a: 0x01,
                    f: 0xB0,
                    b: 0x00,
                    c: 0x13,
                    h: 0x01,
                    l: 0x4D,
                    ..Default::default()
                },
                mmu,
                ime: true,
                timer_counter: 0,
                divider_counter: 0,
                scanline_counter: 0,
            }
        }

        pub fn load_rom(&mut self, rom: &[u8]) {
            self.mmu.load_rom(rom);
        }

        pub fn registers(&self) -> &Registers {
            &self.registers
        }

        pub fn read_byte(&self, addr: u16) -> u8 {
            self.mmu.read_byte(addr)
        }

        pub fn write_byte(&mut self, addr: u16, value: u8) {
            self.mmu.write_byte(addr, value)
        }

        fn update_timers(&mut self, cycles: u8) {
            self.divider_counter += cycles as u32;
            if self.divider_counter >= 256 {
                let div = self.read_byte(0xFF04).wrapping_add(1);
                self.write_byte(0xFF04, div);
                self.divider_counter -= 256;
            }

            let tac = self.read_byte(0xFF07);
            if tac & 0x04 != 0 {
                let freq = match tac & 0x03 {
                    0 => 1024,
                    1 => 16,
                    2 => 64,
                    3 => 256,
                    _ => 1024,
                };
                self.timer_counter += cycles as u32;
                if self.timer_counter >= freq {
                    let tima = self.read_byte(0xFF05);
                    if tima == 0xFF {
                        let tma = self.read_byte(0xFF06);
                        self.write_byte(0xFF05, tma);
                        let if_reg = self.read_byte(0xFF0F) | 0x04;
                        self.write_byte(0xFF0F, if_reg);
                    } else {
                        self.write_byte(0xFF05, tima.wrapping_add(1));
                    }
                    self.timer_counter -= freq;
                }
            }
        }

        fn handle_interrupt(&mut self, addr: u16, flag: u8) {
            self.ime = false;
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.write_byte(self.registers.sp, self.registers.pc as u8);
            self.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = addr;
            self.write_byte(0xFF0F, self.read_byte(0xFF0F) & !flag);
        }

        fn update_ppu(&mut self, cycles: u8) {
            let lcdc = self.read_byte(0xFF40);
            if lcdc & 0x80 == 0 {
                println!("PPU disabled: LCDC=0x{:02X}", lcdc);
                self.write_byte(0xFF44, 0);
                self.scanline_counter = 0;
                self.write_byte(0xFF41, self.read_byte(0xFF41) & 0xFC);
                return;
            }

            self.scanline_counter += cycles as u32;
            println!("PPU cycles added: {}, scanline_counter: {}", cycles, self.scanline_counter);
            let mut ly = self.read_byte(0xFF44);
            let mut stat = self.read_byte(0xFF41);
            let mode = stat & 0x03;

            if ly < 144 {
                if self.scanline_counter < 80 {
                    if mode != 2 {
                        stat = (stat & 0xFC) | 0x02;
                        if stat & 0x40 != 0 {
                            self.write_byte(0xFF0F, self.read_byte(0xFF0F) | 0x02);
                        }
                        self.write_byte(0xFF41, stat);
                    }
                } else if self.scanline_counter < 252 {
                    if mode != 3 {
                        stat = (stat & 0xFC) | 0x03;
                        self.write_byte(0xFF41, stat);
                    }
                } else if self.scanline_counter < 456 {
                    if mode != 0 {
                        stat = (stat & 0xFC) | 0x00;
                        if stat & 0x10 != 0 {
                            self.write_byte(0xFF0F, self.read_byte(0xFF0F) | 0x02);
                        }
                        self.write_byte(0xFF41, stat);
                    }
                }
            } else {
                if mode != 1 {
                    stat = (stat & 0xFC) | 0x01;
                    if stat & 0x08 != 0 {
                        self.write_byte(0xFF0F, self.read_byte(0xFF0F) | 0x02);
                    }
                    self.write_byte(0xFF41, stat);
                    if ly == 144 {
                        self.write_byte(0xFF0F, self.read_byte(0xFF0F) | 0x01);
                        println!("VBlank interrupt forced at LY=0x{:02X}", ly);
                    }
                }
            }

           if self.scanline_counter >= 456 {
                self.scanline_counter -= 456;
                ly = ly.wrapping_add(1);
                if ly > 153 {
                    ly = 0;
                }
                self.write_byte(0xFF44, ly);
                println!("LY incremented to 0x{:02X}, scanline_counter: {}", ly, self.scanline_counter);

                if ly == self.read_byte(0xFF45) && (stat & 0x20) != 0 {
                    self.write_byte(0xFF0F, self.read_byte(0xFF0F) | 0x02);
                }
                self.write_byte(0xFF41, stat);
            }
            println!("PPU mode: {}, LY: 0x{:02X}, STAT: 0x{:02X}", mode, ly, stat);
        }

        pub fn step(&mut self) -> u8 {
            let opcode = self.read_byte(self.registers.pc);
            let pc = self.registers.pc;
            self.registers.pc = self.registers.pc.wrapping_add(1);
            println!("INS: 0x{:02X} at PC: 0x{:04X}", opcode, pc);
            if pc >= 0x019B && pc <= 0x01A0 {
                println!("INS: 0x{:02X} at PC: 0x{:04X}, A: 0x{:02X}, F: {:#04x}, HL: 0x{:02X}{:02X}", 
                    opcode, pc, self.registers.a, self.registers.f, self.registers.h, self.registers.l);
            }
            let cycles = match opcode {
                0x00 => 4, // NOP
                0x01 => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.b = high;
                    self.registers.c = low;
                    12
                }
                0x04 => {
                    self.registers.b = self.registers.b.wrapping_add(1);
                    self.registers.f &= 0x10;
                    if self.registers.b == 0 { self.registers.f |= 0x80; }
                    if (self.registers.b & 0x0F) == 0 { self.registers.f |= 0x20; }
                    4
                }
                0x05 => {
                    self.registers.b = self.registers.b.wrapping_sub(1);
                    self.registers.f &= 0x10;
                    if self.registers.b == 0 { self.registers.f |= 0x80; }
                    if (self.registers.b & 0x0F) == 0x0F { self.registers.f |= 0x20; }
                    4
                }
                0x06 => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.b = value;
                    8
                }
                0x0C => {
                    self.registers.c = self.registers.c.wrapping_add(1);
                    self.registers.f &= 0x10;
                    if self.registers.c == 0 { self.registers.f |= 0x80; }
                    if (self.registers.c & 0x0F) == 0 { self.registers.f |= 0x20; }
                    4
                }
                0x0E => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc += 1;
                    self.registers.c = value;
                    8
                }
                0x10 => {
                    // STOP
                    self.write_byte(0xFF0F, self.read_byte(0xFF0F) | 0x10);
                    4
                }
                0x11 => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.d = high;
                    self.registers.e = low;
                    12
                }
                0x18 => {
                    let offset = self.read_byte(self.registers.pc) as i8;
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                    12
                }
                0x1A => {
                    let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                    self.registers.a = self.read_byte(addr);
                    8
                }
                0x20 => {
                    let offset = self.read_byte(self.registers.pc) as i8;
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    if self.registers.f & 0x80 == 0 {
                        println!("JR NZ offset: 0x{:02X}, F: {:#04x}, PC: 0x{:04X}", offset, self.registers.f, self.registers.pc);
                        self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                        12
                    } else {
                        8
                    }
                }
                0x21 => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.h = high;
                    self.registers.l = low;
                    12
                }
                0x22 => {
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.write_byte(addr, self.registers.a);
                    let result = addr.wrapping_add(1);
                    self.registers.h = (result >> 8) as u8;
                    self.registers.l = result as u8;
                    8
                }
                0x23 => {
                    let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    let result = hl.wrapping_add(1);
                    self.registers.h = (result >> 8) as u8;
                    self.registers.l = result as u8;
                    8
                }
                0x28 => {
                    let offset = self.read_byte(self.registers.pc) as i8;
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    if self.registers.f & 0x80 != 0 {
                        self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
                        12
                    } else {
                        8
                    }
                }
                0x2A => {
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.registers.a = self.read_byte(addr);
                    let result = addr.wrapping_add(1);
                    self.registers.h = (result >> 8) as u8;
                    self.registers.l = result as u8;
                    8
                }
                0x2E => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.l = value;
                    8
                }
                0x2F => {
                    // CPL
                    self.registers.a = !self.registers.a;
                    self.registers.f |= 0x60; // 設置 N 和 H 標誌
                    4
                }
                0x32 => {
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.write_byte(addr, self.registers.a);
                    let result = addr.wrapping_sub(1);
                    self.registers.h = (result >> 8) as u8;
                    self.registers.l = result as u8;
                    8
                }
                0x36 => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.write_byte(addr, value);
                    12
                }
                0x3A => {
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.registers.a = self.read_byte(addr);
                    let result = addr.wrapping_sub(1);
                    self.registers.h = (result >> 8) as u8;
                    self.registers.l = result as u8;
                    8
                }
                0x3C => {
                    self.registers.a = self.registers.a.wrapping_add(1);
                    self.registers.f &= 0x10;
                    if self.registers.a == 0 { self.registers.f |= 0x80; }
                    if (self.registers.a & 0x0F) == 0 { self.registers.f |= 0x20; }
                    4
                }
                0x3E => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.a = value;
                    8
                }
                0x40 => {
                    self.registers.b = self.registers.b;
                    4
                }
                0x41 => {
                    self.registers.b = self.registers.c;
                    4
                }
                0x47 => {
                    self.registers.b = self.registers.a;
                    4
                }
                0x77 => {
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.write_byte(addr, self.registers.a);
                    8
                }
                0x78 => {
                    self.registers.a = self.registers.b;
                    4
                }
                0x7E => {
                    let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                    self.registers.a = self.read_byte(addr);
                    8
                }
                0x87 => {
                    self.registers.a &= !0x01;
                    8
                }
                0x94 => {
                    let result = self.registers.a.wrapping_sub(self.registers.h);
                    self.registers.f = 0;
                    if result == 0 { self.registers.f |= 0x80; }
                    self.registers.f |= 0x40;
                    if (self.registers.a & 0x0F) < (self.registers.h & 0x0F) { self.registers.f |= 0x20; }
                    if self.registers.a < self.registers.h { self.registers.f |= 0x10; }
                    self.registers.a = result;
                    4
                }
                0xAF => {
                    self.registers.a = 0;
                    self.registers.f = 0x80;
                    4
                }
                0xB0 => {
                    self.registers.a |= self.registers.b;
                    self.registers.f = 0;
                    if self.registers.a == 0 { self.registers.f |= 0x80; }
                    4
                }
                0xB8 => {
                    let result = self.registers.a.wrapping_sub(self.registers.b);
                    self.registers.f = 0;
                    if result == 0 { self.registers.f |= 0x80; }
                    self.registers.f |= 0x40;
                    if (self.registers.a & 0x0F) < (self.registers.b & 0x0F) { self.registers.f |= 0x20; }
                    if self.registers.a < self.registers.b { self.registers.f |= 0x10; }
                    4
                }
                0xC3 => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = ((high as u16) << 8) | (low as u16);
                    16
                }
                0xC6 => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let result = self.registers.a.wrapping_add(value);
                    self.registers.f = 0;
                    if result == 0 { self.registers.f |= 0x80; }
                    if (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F { self.registers.f |= 0x20; }
                    if (self.registers.a as u16) + (value as u16) > 0xFF { self.registers.f |= 0x10; }
                    self.registers.a = result;
                    8
                }
                0xC9 => {
                    let low = self.read_byte(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    let high = self.read_byte(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.registers.pc = ((high as u16) << 8) | (low as u16);
                    16
                }
                0xCB => {
                    let cb_opcode = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    match cb_opcode {
                        0x00 => {
                            // RLC B
                            let carry = (self.registers.b & 0x80) >> 7;
                            self.registers.b = (self.registers.b << 1) | carry;
                            self.registers.f = 0;
                            if self.registers.b == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x10 => {
                            // RL B
                            let carry = (self.registers.b & 0x80) >> 7;
                            self.registers.b = (self.registers.b << 1) | (self.registers.f & 0x10 >> 4);
                            self.registers.f = 0;
                            if self.registers.b == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x11 => {
                            // RL C
                            let carry = (self.registers.c & 0x80) >> 7;
                            self.registers.c = (self.registers.c << 1) | (self.registers.f & 0x10 >> 4);
                            self.registers.f = 0;
                            if self.registers.c == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x12 => {
                            let carry = (self.registers.d & 0x80) >> 7;
                            self.registers.d = ((self.registers.d as u8) << 1) | (self.registers.f & 0x10 >> 4);
                            self.registers.f = 0;
                            if self.registers.d == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x13 => {
                            let carry = (self.registers.e & 0x80) >> 7;
                            self.registers.e = (self.registers.e << 1) | (self.registers.f & 0x10 >> 4);
                            self.registers.f = 0;
                            if self.registers.e == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x15 => {
                            let carry = (self.registers.l & 0x80) >> 7;
                            self.registers.l = (self.registers.l << 1) | (self.registers.f & 0x10 >> 4);
                            self.registers.f = 0;
                            if self.registers.l == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x1C => {
                            let carry = self.registers.h & 0x01;
                            self.registers.h = (self.registers.h >> 1) | ((self.registers.f & 0x10) << 3);
                            self.registers.f = 0;
                            if self.registers.h == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x1E => {
                            let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                            let value = self.read_byte(addr);
                            let carry = value & 0x01;
                            let result = (value >> 1) | ((self.registers.f & 0x10) << 3);
                            self.write_byte(addr, result);
                            self.registers.f = 0;
                            if result == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            16
                        }
                        0x38 => {
                            // SRL B
                            let carry = self.registers.b & 0x01;
                            self.registers.b >>= 1;
                            self.registers.f = 0;
                            if self.registers.b == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x39 => {
                            // SRL C
                            let carry = self.registers.c & 0x01;
                            self.registers.c >>= 1;
                            self.registers.f = 0;
                            if self.registers.c == 0 { self.registers.f |= 0x80; }
                            if carry != 0 { self.registers.f |= 0x10; }
                            8
                        }
                        0x7C => {
                            // BIT 7,H
                            self.registers.f = if self.registers.h & 0x80 == 0 { 0x80 } else { 0x00 };
                            self.registers.f |= 0x20;
                            8
                        }
                        0xC7 => {
                            // SET 0,A
                            self.registers.a |= 0x01;
                            8
                        }
                        _ => {
                            println!("未知 CB 指令: 0x{:02X} at PC: 0x{:04X}", cb_opcode, pc);
                            8
                        }
                    }
                }
                0xCD => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.registers.sp = self.registers.sp.wrapping_sub(2);
                    let ret_addr = self.registers.pc;
                    self.write_byte(self.registers.sp + 1, (ret_addr >> 8) as u8);
                    self.write_byte(self.registers.sp, ret_addr as u8);
                    self.registers.pc = ((high as u16) << 8) | (low as u16);
                    24
                }
                0xD6 => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let result = self.registers.a.wrapping_sub(value);
                    self.registers.f = 0;
                    if result == 0 { self.registers.f |= 0x80; }
                    self.registers.f |= 0x40;
                    if (self.registers.a & 0x0F) < (value & 0x0F) { self.registers.f |= 0x20; }
                    if self.registers.a < value { self.registers.f |= 0x10; }
                    self.registers.a = result;
                    8
                }
                0xE0 => {
                    let offset = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    self.write_byte(0xFF00 + offset as u16, self.registers.a);
                    12
                }
                0xE1 => {
                    let low = self.read_byte(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    let high = self.read_byte(self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.registers.h = high;
                    self.registers.l = low;
                    12
                }
                0xE2 => {
                    // LD (C), A
                    let addr = 0xFF00 + self.registers.c as u16;
                    self.write_byte(addr, self.registers.a);
                    8
                }
                0xEA => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let addr = ((high as u16) << 8) | (low as u16);
                    self.write_byte(addr, self.registers.a);
                    16
                }
                0xF0 => {
                    let offset = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let addr = 0xFF00 + offset as u16;
                    self.registers.a = self.read_byte(addr);
                    if pc == 0x019B {
                        println!("LD A, (0xFF00+0x{:02X}) = 0x{:02X}", offset, self.registers.a);
                    }
                    12
                }
                0xF2 => {
                    let addr = 0xFF00 + self.registers.c as u16;
                    self.registers.a = self.read_byte(addr);
                    8
                }
                0xF3 => {
                    self.ime = false;
                    4
                }
                0xFA => {
                    let low = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let high = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let addr = ((high as u16) << 8) | (low as u16);
                    self.registers.a = self.read_byte(addr);
                    16
                }
                0xFB => {
                    self.ime = true;
                    4
                }
                0xFE => {
                    let value = self.read_byte(self.registers.pc);
                    self.registers.pc = self.registers.pc.wrapping_add(1);
                    let result = self.registers.a.wrapping_sub(value);
                    self.registers.f = 0;
                    if result == 0 { self.registers.f |= 0x80; }
                    self.registers.f |= 0x40;
                    if (self.registers.a & 0x0F) < (value & 0x0F) { self.registers.f |= 0x20; }
                    if self.registers.a < value { self.registers.f |= 0x10; }
                    if pc == 0x019D {
                        println!("CP 0x{:02X}, A: 0x{:02X}, F: {:#04x}", value, self.registers.a, self.registers.f);
                    }
                    8
                }
                _ => {
                    println!("未知指令: 0x{:02X} at PC: 0x{:04X}", opcode, pc);
                    4
                }
            };
            self.update_timers(cycles);
            self.update_ppu(80);
            if self.ime && self.read_byte(0xFF0F) & self.read_byte(0xFFFF) != 0 {
                let if_reg = self.read_byte(0xFF0F);
                let ie_reg = self.read_byte(0xFFFF);
                let interrupts = if_reg & ie_reg;
                if interrupts != 0 {
                    println!("Interrupt triggered: IF=0x{:02X}, IE=0x{:02X}, IME={}", if_reg, ie_reg, self.ime);
                }
                if interrupts & 0x01 != 0 {
                    println!("VBlank interrupt at PC=0x{:04X}", self.registers.pc);
                    self.handle_interrupt(0x0040, 0x01);
                    return 20;
                }
                if interrupts & 0x02 != 0 {
                    self.handle_interrupt(0x0048, 0x02);
                    return 20;
                }
                if interrupts & 0x04 != 0 {
                    self.handle_interrupt(0x0050, 0x04);
                    return 20;
                }
                if interrupts & 0x10 != 0 {
                    println!("JOYPAD interrupt at PC=0x{:04X}", self.registers.pc);
                    self.handle_interrupt(0x0060, 0x10);
                    return 20;
                }
            }
            cycles
        }
    }
}

mod mmu {
    pub struct MMU {
        memory: [u8; 0x10000],
        rom: Vec<u8>,
        ram: [u8; 0x8000],
        rom_bank: u8,
        ram_bank: u8,
        ram_enabled: bool,
        banking_mode: u8,
    }

    impl MMU {
        pub fn new() -> Self {
            MMU {
                memory: [0; 0x10000],
                rom: Vec::new(),
                ram: [0; 0x8000],
                rom_bank: 1,
                ram_bank: 0,
                ram_enabled: false,
                banking_mode: 0,
            }
        }

        pub fn read_byte(&self, addr: u16) -> u8 {
            match addr {
                0x0000..=0x3FFF => self.memory[addr as usize],
                0x4000..=0x7FFF => {
                    let bank = if self.banking_mode == 0 {
                        self.rom_bank
                    } else {
                        (self.ram_bank << 5) | (self.rom_bank & 0x1F)
                    };
                    let offset = (bank as usize * 0x4000) + (addr as usize - 0x4000);
                    let value = self.rom.get(offset).copied().unwrap_or(0);
                    println!("ROM read: addr=0x{:04X}, bank=0x{:02X}, offset=0x{:04X}, value=0x{:02X}", addr, bank, offset, value);
                    value
                }
                0xA000..=0xBFFF => {
                    if self.ram_enabled {
                        let offset = (self.ram_bank as usize * 0x2000) + (addr as usize - 0xA000);
                        self.ram[offset]
                    } else {
                        0xFF
                    }
                }
                _ => self.memory[addr as usize],
            }
        }

        pub fn write_byte(&mut self, addr: u16, value: u8) {
            match addr {
                0x0000..=0x1FFF => {
                    self.ram_enabled = (value & 0x0A) == 0x0A;
                }
                0x2000..=0x3FFF => {
                    let bank = value & 0x1F;
                    self.rom_bank = if bank == 0 { 1 } else { bank };
                }
                0x4000..=0x5FFF => {
                    self.ram_bank = value & 0x03;
                }
                0x6000..=0x7FFF => {
                    self.banking_mode = value & 0x01;
                }
                0x8000..=0x9FFF => {
                    // if addr >= 0x9800 {
                    //     println!("Tile map write 0x{:04X} = 0x{:02X}", addr, value);
                    // } else {
                    //     println!("VRAM write 0x{:04X} = 0x{:02X}", addr, value);
                    // }
                    self.memory[addr as usize] = value;
                }
                0xFF40 => {
                    println!("LCDC write: 0x{:02X}", value);
                    self.memory[addr as usize] = value;
                }
                0xFF47 => {
                    self.memory[addr as usize] = if value == 0 { 0xFC } else { value };
                }
                0xFF48 => {
                    self.memory[addr as usize] = if value == 0 { 0xFF } else { value };
                }
                0xFF49 => {
                    self.memory[addr as usize] = if value == 0 { 0xFF } else { value };
                }
                0xFF44 => {}
                _ => self.memory[addr as usize] = value,
            }
        }       
        pub fn load_rom(&mut self, rom: &[u8]) {
            if rom.len() < 0x150 {
                panic!("ROM too small: {} bytes", rom.len());
            }
            self.rom = rom.to_vec();
            for (i, &byte) in rom.iter().enumerate().take(0x8000) {
                self.memory[i] = byte;
            }
            let title = self.memory[0x134..0x143]
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c as char)
                .collect::<String>();
            let cartridge_type = self.memory[0x147];
            println!("ROM 標題: {}, 卡匣類型: 0x{:02X}", title, cartridge_type);
            if cartridge_type != 0x00 && cartridge_type != 0x01 {
                println!("警告：僅支援無 MBC 或 MBC1，當前類型: 0x{:02X}", cartridge_type);
            }
        }
    }
}


fn main() {
    let mmu = mmu::MMU::new();
    let mut cpu = cpu::CPU::new(mmu);
    let rom_path = std::env::args().nth(1).unwrap_or("test.gb".to_string());
    let rom = fs::read(&rom_path).unwrap_or_else(|e| {
        eprintln!("無法讀取 {}: {}. 使用預設 ROM。", rom_path, e);
        vec![0; 0x150]
    });
    cpu.load_rom(&rom);

    // 任天堂 Logo bytes
    let nintendo_logo: [u8; 48] = [
        0xCE,0xED,0x66,0x66,0xCC,0x0D,0x00,0x0B,
        0x03,0x73,0x00,0x83,0x00,0x0C,0x00,0x0D,
        0x00,0x08,0x11,0x1F,0x88,0x89,0x00,0x0E,
        0xDC,0xCC,0x6E,0xE6,0xDD,0xDD,0xD9,0x99,
        0xBB,0xBB,0x67,0x63,0x6E,0x0E,0xEC,0xCC,
        0xDD,0xDC,0x99,0x9F,0xBB,0xB9,0x33,0x3E,
    ];
    for (i, &b) in nintendo_logo.iter().enumerate() {
        cpu.write_byte(0x0104 + i as u16, b);
    }

    cpu.write_byte(0xFF05, 0x00); // TIMA
    cpu.write_byte(0xFF06, 0x00); // TMA
    cpu.write_byte(0xFF07, 0x00); // TAC
    cpu.write_byte(0xFF10, 0x80);
    cpu.write_byte(0xFF11, 0xBF);
    cpu.write_byte(0xFF12, 0xF3);
    cpu.write_byte(0xFF14, 0xBF);
    cpu.write_byte(0xFF16, 0x3F);
    cpu.write_byte(0xFF17, 0x00);
    cpu.write_byte(0xFF19, 0xBF);
    cpu.write_byte(0xFF1A, 0x7F);
    cpu.write_byte(0xFF1B, 0xFF);
    cpu.write_byte(0xFF1C, 0x9F);
    cpu.write_byte(0xFF1E, 0xBF);
    cpu.write_byte(0xFF20, 0xFF);
    cpu.write_byte(0xFF21, 0x00);
    cpu.write_byte(0xFF22, 0x00);
    cpu.write_byte(0xFF23, 0xBF);
    cpu.write_byte(0xFF24, 0x77);
    cpu.write_byte(0xFF25, 0xF3);
    cpu.write_byte(0xFF26, 0xF1); // GB, 0xF0 for SGB
    cpu.write_byte(0xFF40, 0x91);
    cpu.write_byte(0xFF42, 0x00);
    cpu.write_byte(0xFF43, 0x00);
    cpu.write_byte(0xFF45, 0x00);
    cpu.write_byte(0xFF47, 0xFC);
    cpu.write_byte(0xFF48, 0xFF);
    cpu.write_byte(0xFF49, 0xFF);
    cpu.write_byte(0xFF4A, 0x00);
    cpu.write_byte(0xFF4B, 0x00);
    cpu.write_byte(0xFFFF, 0x00); // IE

    cpu.write_byte(0xFF40, 0x91);

    // 新增：初始化 tile map 0x9800 和 VRAM 0x8000 前 16 bytes
    for i in 0..16 {
        cpu.write_byte(0x9800 + i, (i + 1) as u8);   // 0x01 ~ 0x10
        cpu.write_byte(0x8000 + i, (0x11 + i) as u8); // 0x11 ~ 0x20
    }

    let mut window = Window::new("Game Boy Emulator", 160, 144, WindowOptions::default())
        .unwrap_or_else(|e| panic!("視窗創建失敗: {}", e));
    let mut buffer: Vec<u32> = vec![0; 160 * 144];
    let mut frame_count = 0;
    let mut total_cycles = 0;
    let _total_cycles = 0;
    

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut frame_cycles = 0;
        while frame_cycles < 70224 {
            let cycles = cpu.step();
            frame_cycles += cycles as u32;
            total_cycles += cycles as u32;
        }

        frame_count += 1;
        if frame_count % 60 == 0 {
            let regs = cpu.registers();
            println!(
                "PC: 0x{:04X}, A: 0x{:02X}, F: {:#04x}, BC: 0x{:02X}{:02X}, DE: 0x{:02X}{:02X}, HL: 0x{:02X}{:02X}",
                regs.pc, regs.a, regs.f, regs.b, regs.c, regs.d, regs.e, regs.h, regs.l
            );
            println!(
                "DIV: 0x{:02X}, TIMA: 0x{:02X}, TMA: 0x{:02X}, TAC: 0x{:02X}",
                cpu.read_byte(0xFF04),
                cpu.read_byte(0xFF05),
                cpu.read_byte(0xFF06),
                cpu.read_byte(0xFF07)
            );
            println!("LY: 0x{:02X}, STAT: {:#04x}, LCDC: 0x{:02X}, Scanline counter: {}", 
                     cpu.read_byte(0xFF44), cpu.read_byte(0xFF41), cpu.read_byte(0xFF40), cpu.scanline_counter);
            println!("Tile map 0x9800[0]: 0x{:02X}, VRAM 0x8000[0]: 0x{:02X}", 
                     cpu.read_byte(0x9800), cpu.read_byte(0x8000));
            println!("Total cycles this frame: {}", frame_cycles);

            // palette 狀態
            println!(
                "BGP: 0x{:02X}, OBP0: 0x{:02X}, OBP1: 0x{:02X}",
                cpu.read_byte(0xFF47),
                cpu.read_byte(0xFF48),
                cpu.read_byte(0xFF49)
            );

            // tile map 狀態
            println!(
                "Tile map 0x9800[0]: 0x{:02X}, VRAM 0x8000[0]: 0x{:02X}",
                cpu.read_byte(0x9800),
                cpu.read_byte(0x8000)
            );

            for i in 0..16 {
                print!("{:02X} ", cpu.read_byte(0x9800 + i));
            }
            println!();
        }

        if frame_count >= 100 && frame_count <= 150 {
            println!("Simulating Start button press at frame {}", frame_count);
            let mut joypad = cpu.read_byte(0xFF00);
            joypad &= !0x08;
            cpu.write_byte(0xFF00, joypad);
            cpu.write_byte(0xFF0F, cpu.read_byte(0xFF0F) | 0x10);
        }

        let lcd_control = cpu.read_byte(0xFF40);
        if lcd_control & 0x80 != 0 {
            let scroll_y = cpu.read_byte(0xFF42) as u16;
            let scroll_x = cpu.read_byte(0xFF43) as u16;
            let bgp = cpu.read_byte(0xFF47);
            let colors = [
                0xFFFFFF,
                0xAAAAAA,
                0x555555,
                0x000000,
            ];

            if frame_count % 60 == 0 {
                println!(
                    "LCDC: {:#04x}, BGP: {:#02x}, OBP0: {:#02x}, OBP1: {:#02x}",
                    lcd_control, bgp, cpu.read_byte(0xFF48), cpu.read_byte(0xFF49)
                );
                println!("Total cycles: {}", total_cycles); // 修正這一行
            }

            if lcd_control & 0x01 != 0 {
                let tile_map = if lcd_control & 0x08 == 0 { 0x9800 } else { 0x9C00 };
                for tile_y in 0..18u16 {
                    for tile_x in 0..20u16 {
                        let tile_idx = cpu.read_byte(tile_map + tile_y * 32 + tile_x) as u16;
                        let tile_addr = if lcd_control & 0x10 == 0 {
                            let idx = tile_idx as i16;
                            if idx >= 128 {
                                0x8800 + ((idx - 128) as u16 * 16)
                            } else {
                                0x9000 + (idx as u16 * 16)
                            }
                        } else {
                            0x8000 + tile_idx * 16
                        };
                        for y in 0..8u16 {
                            let row1 = cpu.read_byte(tile_addr + y * 2);
                            let row2 = cpu.read_byte(tile_addr + y * 2 + 1);
                            for x in 0..8u16 {
                                let bit1 = (row1 >> (7 - x)) & 1;
                                let bit2 = (row2 >> (7 - x)) & 1;
                                let color_idx = (bit2 << 1) | bit1;
                                let color = colors[((bgp >> (color_idx * 2)) & 0x03) as usize];
                                let px = (tile_x * 8 + x + scroll_x) % 256;
                                let py = (tile_y * 8 + y + scroll_y) % 256;
                                if px < 160 && py < 144 {
                                    let idx = (py as usize) * 160 + (px as usize);
                                    buffer[idx] = color;
                                }
                            }
                        }
                    }
                }
            }

            if lcd_control & 0x02 != 0 {
                let sprite_height = if lcd_control & 0x04 != 0 { 16 } else { 8 };
                for sprite in 0..40 {
                    let sprite_addr = 0xFE00 + sprite * 4;
                    let y = cpu.read_byte(sprite_addr) as i16 - 16;
                    let x = cpu.read_byte(sprite_addr + 1) as i16 - 8;
                    let tile_idx = if sprite_height == 16 {
                        cpu.read_byte(sprite_addr + 2) as u16 & 0xFE
                    } else {
                        cpu.read_byte(sprite_addr + 2) as u16
                    };
                    let flags = cpu.read_byte(sprite_addr + 3);
                    let palette = if flags & 0x10 != 0 {
                        cpu.read_byte(0xFF49)
                    } else {
                        cpu.read_byte(0xFF48)
                    };
                    let y_flip = flags & 0x40 != 0;
                    let x_flip = flags & 0x20 != 0;
                    let tile_addr = 0x8000 + (tile_idx * 16);
                    for py in 0..sprite_height {
                        let py_adj = if y_flip { sprite_height - 1 - py } else { py };
                        let row1 = cpu.read_byte(tile_addr + py_adj * 2);
                        let row2 = cpu.read_byte(tile_addr + py_adj * 2 + 1);
                        for px in 0..8 {
                            let px_adj = if x_flip { 7 - px } else { px };
                            let bit1 = (row1 >> (7 - px_adj)) & 1;
                            let bit2 = (row2 >> (7 - px_adj)) & 1;
                            let color_idx = (bit2 << 1) | bit1;
                            if color_idx != 0 {
                                let color = colors[((palette >> (color_idx * 2)) & 0x03) as usize];
                                let px = x + px as i16;
                                let py = y + py as i16;
                                if px >= 0 && px < 160 && py >= 0 && py < 144 {
                                    let idx = (py as usize) * 160 + (px as usize);
                                    if flags & 0x80 == 0 || buffer[idx] == colors[0] {
                                        buffer[idx] = color;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if lcd_control & 0x20 != 0 {
                let wy = cpu.read_byte(0xFF4A) as u16;
                let wx = cpu.read_byte(0xFF4B) as u16;
                let tile_map = if lcd_control & 0x40 != 0 { 0x9C00 } else { 0x9800 };
                for tile_y in 0..18u16 {
                    for tile_x in 0..20u16 {
                        let tile_idx = cpu.read_byte(tile_map + tile_y * 32 + tile_x) as u16;
                        let tile_addr = if lcd_control & 0x10 == 0 {
                            let idx = tile_idx as i16;
                            if idx >= 128 {
                                0x8800 + ((idx - 128) as u16 * 16)
                            } else {
                                0x9000 + (idx as u16 * 16)
                            }
                        } else {
                            0x8000 + tile_idx * 16
                        };
                        for y in 0..8u16 {
                            let row1 = cpu.read_byte(tile_addr + y * 2);
                            let row2 = cpu.read_byte(tile_addr + y * 2 + 1);
                            for x in 0..8 {
                                let bit1 = (row1 >> (7 - x)) & 1;
                                let bit2 = (row2 >> (7 - x)) & 1;
                                let color_idx = (bit2 << 1) | bit1;
                                let color = colors[((bgp >> (color_idx * 2)) & 0x03) as usize];
                                let px = wx + tile_x * 8 + x - 7;
                                let py = wy + tile_y * 8 + y;
                                if px < 160 && py < 144 {
                                    let idx = (py as usize) * 160 + (px as usize);
                                    buffer[idx] = color;
                                }
                            }
                        }
                    }
                }
            }
        } 
        // else {
            //buffer.fill(0xFFFFFF);
           // buffer.fill(0xFF0000);
        // }

        // 放在這裡，每 frame 印一次 palette 狀態
        println!(
            "BGP: 0x{:02X}, OBP0: 0x{:02X}, OBP1: 0x{:02X}",
            cpu.read_byte(0xFF47),
            cpu.read_byte(0xFF48),
            cpu.read_byte(0xFF49)
        );

        // 加在這裡
        println!(
            "Tile map 0x9800[0]: 0x{:02X}, VRAM 0x8000[0]: 0x{:02X}",
            cpu.read_byte(0x9800),
            cpu.read_byte(0x8000)
        );

        for i in 0..16 {
            print!("{:02X} ", cpu.read_byte(0x9800 + i));
        }
        println!();

        // 新增：讓 tile map 0x9800[0] 與 VRAM 0x8000[0] 每 frame 遞增
        let v = cpu.read_byte(0x9800);
        cpu.write_byte(0x9800, v.wrapping_add(1));
        let v = cpu.read_byte(0x8000);
        cpu.write_byte(0x8000, v.wrapping_add(1));

        let joypad_select = cpu.read_byte(0xFF00);
        let mut joypad = 0xFF;
        if joypad_select & 0x10 == 0 {
            // 方向鍵
            if window.is_key_down(Key::Right) { joypad &= !0x01; }
            if window.is_key_down(Key::Left) { joypad &= !0x02; }
            if window.is_key_down(Key::Up) { joypad &= !0x04; }
            if window.is_key_down(Key::Down) { joypad &= !0x08; }
        }
        if joypad_select & 0x20 == 0 {
            // 動作鍵
            if window.is_key_down(Key::Z) { joypad &= !0x01; } // A
            if window.is_key_down(Key::X) { joypad &= !0x02; } // B
            if window.is_key_down(Key::Space) { joypad &= !0x04; } // Select
            if window.is_key_down(Key::Enter) { joypad &= !0x08; } // Start
        }
        joypad |= joypad_select & 0x30; // 保留 P14/P15
        cpu.write_byte(0xFF00, joypad);
        if joypad != 0xFF {
            cpu.write_byte(0xFF0F, cpu.read_byte(0xFF0F) | 0x10); // 觸發 JOYPAD 中斷
        }
        window.update_with_buffer(&buffer, 160, 144).unwrap();
    }
}//

