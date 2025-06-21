use crate::core::cpu::instructions::register_utils::FlagOperations;
use crate::core::cpu::CPU;
use crate::core::cycles::{CyclesType, CYCLES_1, CYCLES_2, CYCLES_3};
use crate::error::{Error, InstructionError, RegTarget, Result};
use std::io::Write;

/// 處理 LD 指令族
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // LD BC, nn
        0x01 => cpu.ld_bc_nn(),

        // LD DE, nn
        0x11 => cpu.ld_de_nn(),

        // LD HL, nn
        0x21 => cpu.ld_hl_nn(),

        // LD SP, nn
        0x31 => cpu.ld_sp_nn(),

        // LD (HL), r
        0x70..=0x77 => {
            let src = opcode & 0x07;
            let source = RegTarget::from_bits(src)?;
            cpu.ld_hl_r(source)
        }

        // LD r, r'
        0x40..=0x7F => {
            let dst = ((opcode >> 3) & 0x07) as u8;
            let src = (opcode & 0x07) as u8;
            let target = RegTarget::from_bits(dst)?;
            let source = RegTarget::from_bits(src)?;
            cpu.ld_r_r(target, source)
        }

        // LD r, n
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
            let reg = ((opcode >> 3) & 0x07) as u8;
            let target = RegTarget::from_bits(reg)?;
            cpu.ld_r_n(target)
        }

        // LD A, (BC)
        0x0A => cpu.ld_a_bc(),

        // LD A, (DE)
        0x1A => cpu.ld_a_de(),

        // LD A, (nn)
        0xFA => cpu.ld_a_nn(),

        // LD (BC), A
        0x02 => cpu.ld_bc_a(),

        // LD (DE), A
        0x12 => cpu.ld_de_a(),

        // LD (nn), A
        0xEA => cpu.ld_nn_a(),

        // LD A, (C)
        0xF2 => cpu.ld_a_c(),

        // LD (C), A
        0xE2 => cpu.ld_c_a(),

        // LDH (n), A
        0xE0 => cpu.ldh_n_a(),

        // LDH A, (n)
        0xF0 => cpu.ldh_a_n(),

        // LD (HL+), A
        0x22 => cpu.ld_hli_a(),

        // LD A, (HL+)
        0x2A => cpu.ld_a_hli(),

        // LD (HL-), A
        0x32 => cpu.ld_hld_a(),

        // LD A, (HL-)
        0x3A => cpu.ld_a_hld(), // LD (HL), n
        0x36 => cpu.ld_hl_n(),

        // LD SP, HL
        0xF9 => cpu.ld_sp_hl(),

        // LD HL, SP+r8
        0xF8 => cpu.ld_hl_sp_r8(),

        // LD (nn),SP
        0x08 => cpu.ld_nn_sp(),

        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

/// 實作 LD 指令相關方法
impl CPU {
    pub fn ld_r_r(&mut self, target: RegTarget, source: RegTarget) -> Result<CyclesType> {
        let value = match source {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    source,
                )))
            }
        };

        match target {
            RegTarget::A => self.registers.a = value,
            RegTarget::B => self.registers.b = value,
            RegTarget::C => self.registers.c = value,
            RegTarget::D => self.registers.d = value,
            RegTarget::E => self.registers.e = value,
            RegTarget::H => self.registers.h = value,
            RegTarget::L => self.registers.l = value,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.write_byte(addr, value)?;
                // 記錄 VRAM 寫入
                self.log_vram_write(addr, value, &format!("Register {:?}", source))?;
            }
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    target,
                )))
            }
        }

        Ok(CYCLES_1)
    }

    pub fn ld_r_n(&mut self, target: RegTarget) -> Result<CyclesType> {
        let value = self.fetch_byte()?;

        match target {
            RegTarget::A => self.registers.a = value,
            RegTarget::B => self.registers.b = value,
            RegTarget::C => self.registers.c = value,
            RegTarget::D => self.registers.d = value,
            RegTarget::E => self.registers.e = value,
            RegTarget::H => self.registers.h = value,
            RegTarget::L => self.registers.l = value,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.write_byte(addr, value)?;
            }
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    target,
                )))
            }
        }

        Ok(CYCLES_2)
    }
    pub fn ld_hl_n(&mut self) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        let addr = self.registers.get_hl();

        self.write_byte(addr, value)?;

        // 記錄 VRAM 寫入
        self.log_vram_write(addr, value, "Immediate")?;

        Ok(CYCLES_2)
    }

    pub fn ld_hli_a(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_hl();
        self.write_byte(addr, self.registers.a)?;
        self.registers.set_hl(addr.wrapping_add(1));
        Ok(CYCLES_2)
    }

    pub fn ld_a_hli(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_hl();
        self.registers.a = self.read_byte(addr)?;
        self.registers.set_hl(addr.wrapping_add(1));
        Ok(CYCLES_2)
    }

    pub fn ld_hld_a(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_hl();
        self.write_byte(addr, self.registers.a)?;
        self.registers.set_hl(addr.wrapping_sub(1));
        Ok(CYCLES_2)
    }

    pub fn ld_a_hld(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_hl();
        self.registers.a = self.read_byte(addr)?;
        self.registers.set_hl(addr.wrapping_sub(1));
        Ok(CYCLES_2)
    }

    pub fn ld_a_bc(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_bc();
        self.registers.a = self.read_byte(addr)?;
        Ok(CYCLES_2)
    }

    pub fn ld_bc_a(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_bc();
        self.write_byte(addr, self.registers.a)?;
        Ok(CYCLES_2)
    }

    pub fn ld_a_de(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_de();
        self.registers.a = self.read_byte(addr)?;
        Ok(CYCLES_2)
    }

    pub fn ld_de_a(&mut self) -> Result<CyclesType> {
        let addr = self.registers.get_de();
        self.write_byte(addr, self.registers.a)?;
        Ok(CYCLES_2)
    }

    pub fn ld_a_nn(&mut self) -> Result<CyclesType> {
        let addr = self.fetch_word()?;
        self.registers.a = self.read_byte(addr)?;
        Ok(CYCLES_3)
    }

    pub fn ld_nn_a(&mut self) -> Result<CyclesType> {
        let addr = self.fetch_word()?;
        self.write_byte(addr, self.registers.a)?;
        Ok(CYCLES_3)
    }

    pub fn ld_a_c(&mut self) -> Result<CyclesType> {
        let addr = 0xFF00 | (self.registers.c as u16);
        self.registers.a = self.read_byte(addr)?;
        Ok(CYCLES_2)
    }

    pub fn ld_c_a(&mut self) -> Result<CyclesType> {
        let addr = 0xFF00 | (self.registers.c as u16);
        self.write_byte(addr, self.registers.a)?;
        Ok(CYCLES_2)
    }

    pub fn ldh_n_a(&mut self) -> Result<CyclesType> {
        let offset = self.fetch_byte()?;
        let addr = 0xFF00 | (offset as u16);
        self.write_byte(addr, self.registers.a)?;
        Ok(CYCLES_2)
    }

    pub fn ldh_a_n(&mut self) -> Result<CyclesType> {
        let offset = self.fetch_byte()?;
        let addr = 0xFF00 | (offset as u16);
        self.registers.a = self.read_byte(addr)?;
        Ok(CYCLES_2)
    }

    pub fn ld_bc_nn(&mut self) -> Result<CyclesType> {
        let nn = self.fetch_word()?;
        self.registers.set_bc(nn);
        Ok(CYCLES_3)
    }

    pub fn ld_de_nn(&mut self) -> Result<CyclesType> {
        let nn = self.fetch_word()?;
        self.registers.set_de(nn);
        Ok(CYCLES_3)
    }

    pub fn ld_hl_nn(&mut self) -> Result<CyclesType> {
        let nn = self.fetch_word()?;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/cpu_exec.log")
        {
            writeln!(
                file,
                "執行 LD HL, 0x{:04X} (PC=0x{:04X})",
                nn,
                self.registers.get_pc()
            )
            .ok();
        }
        self.registers.set_hl(nn);
        Ok(CYCLES_3)
    }

    pub fn ld_sp_nn(&mut self) -> Result<CyclesType> {
        let nn = self.fetch_word()?;
        self.registers.sp = nn;
        Ok(CYCLES_3)
    }

    pub fn ld_sp_hl(&mut self) -> Result<CyclesType> {
        self.registers.sp = self.registers.get_hl();
        Ok(CYCLES_2)
    }

    pub fn ld_hl_sp_r8(&mut self) -> Result<CyclesType> {
        let r8 = self.fetch_byte()? as i8 as i16 as u16;
        let result = self.registers.sp.wrapping_add(r8);
        self.registers.set_hl(result);

        // 設置標誌位
        self.registers.set_zero(false);
        self.registers.set_subtract(false);

        // H 和 C 標誌位的計算
        let half_carry = (self.registers.sp & 0xF) + (r8 & 0xF) > 0xF;
        let carry = (self.registers.sp & 0xFF) + (r8 & 0xFF) > 0xFF;
        self.registers.set_half_carry(half_carry);
        self.registers.set_carry(carry);

        Ok(CYCLES_3)
    }

    pub fn jump(&mut self) -> Result<CyclesType> {
        let addr = self.fetch_word()?;
        self.registers.pc = addr;
        Ok(CYCLES_3)
    }

    /// LD (nn),SP - 將 SP 寫入到 nn 指定的記憶體位置
    pub fn ld_nn_sp(&mut self) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        let sp = self.registers.get_sp();
        let mut mmu = self.mmu.borrow_mut();
        mmu.write_byte(address, (sp & 0xFF) as u8)?;
        mmu.write_byte(address + 1, (sp >> 8) as u8)?;
        Ok(20) // 指令執行需要 20 個時鐘週期
    }
    pub fn ld_hl_r(&mut self, source: RegTarget) -> Result<CyclesType> {
        let value = match source {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            reg => return Err(Error::Instruction(InstructionError::InvalidRegister(reg))),
        };

        let addr = self.registers.get_hl();

        // 寫入記憶體
        self.write_byte(addr, value)?;

        // 記錄 VRAM 寫入
        self.log_vram_write(addr, value, &format!("Register {:?}", source))?;

        Ok(CYCLES_2)
    }

    fn log_vram_write(&self, addr: u16, value: u8, source: &str) -> Result<()> {
        if addr >= 0x8000 && addr <= 0x9FFF {
            let mut log_msg = format!(
                "VRAM Write: addr=0x{:04X}, value=0x{:02X}, src={}, PC=0x{:04X}, ",
                addr, value, source, self.registers.pc
            );

            // 如果在 tile data 區域
            if addr >= 0x8000 && addr <= 0x97FF {
                let tile_number = (addr - 0x8000) / 16;
                let row = ((addr - 0x8000) % 16) / 2;
                let is_high_bits = (addr - 0x8000) % 2 == 1;
                log_msg.push_str(&format!(
                    "Tile Data: tile={}, row={}, {}",
                    tile_number,
                    row,
                    if is_high_bits { "high" } else { "low" }
                ));
            }
            // 如果在 tile map 區域
            else if addr >= 0x9800 && addr <= 0x9FFF {
                let map_number = if addr >= 0x9C00 { 1 } else { 0 };
                let tile_pos = addr - if map_number == 0 { 0x9800 } else { 0x9C00 };
                log_msg.push_str(&format!("Tile Map {}: pos={}", map_number, tile_pos));
            }

            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/vram_write.log")
            {
                writeln!(&mut file, "{}", log_msg)?;
            }
        }
        Ok(())
    }
}
