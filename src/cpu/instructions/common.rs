use crate::error::Error;
use std::fmt;

pub const CYCLES_1: u8 = 4;
pub const CYCLES_2: u8 = 8;
pub const CYCLES_3: u8 = 12;
pub const CYCLES_4: u8 = 16;
pub const CYCLES_5: u8 = 20;
pub const CYCLES_6: u8 = 24;

#[derive(Debug)]
pub enum InstructionError {
    InvalidOpcode(u8),
    InvalidRegister(RegTarget),
    MemoryError(String),
    UnimplementedInstruction(u8),
    Custom(String),
}

impl fmt::Display for InstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionError::InvalidOpcode(opcode) => write!(f, "Invalid opcode: {:#04X}", opcode),
            InstructionError::InvalidRegister(reg) => write!(f, "Invalid register: {:?}", reg),
            InstructionError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            InstructionError::UnimplementedInstruction(opcode) => {
                write!(f, "Unimplemented instruction: {:#04X}", opcode)
            }
            InstructionError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for InstructionError {}

impl From<Error> for InstructionError {
    fn from(error: Error) -> Self {
        match error {
            Error::Memory(msg) => InstructionError::MemoryError(msg),
            Error::VramInaccessible => {
                InstructionError::MemoryError("VRAM inaccessible".to_string())
            }
            Error::OamInaccessible => InstructionError::MemoryError("OAM inaccessible".to_string()),
            _ => InstructionError::Custom(error.to_string()),
        }
    }
}

pub trait FlagOperations {
    fn get_zero_flag(&self) -> bool;
    fn set_zero_flag(&mut self, value: bool);
    fn get_subtract_flag(&self) -> bool;
    fn set_subtract_flag(&mut self, value: bool);
    fn get_half_carry_flag(&self) -> bool;
    fn set_half_carry_flag(&mut self, value: bool);
    fn get_carry_flag(&self) -> bool;
    fn set_carry_flag(&mut self, value: bool);
    fn update_flags(&mut self, z: bool, n: bool, h: bool, c: bool);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegTarget {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HL
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegPair {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
}

#[derive(Debug, Clone, Copy)]
pub enum Condition {
    NZ, // 非零
    Z,  // 零
    NC, // 無進位
    C,  // 進位
}

pub type Result<T> = std::result::Result<T, InstructionError>;

pub mod cycles {
    pub const NOP: u8 = 4;
    pub const LD_R_R: u8 = 4;
    pub const LD_R_N: u8 = 8;
    pub const LD_R_HL: u8 = 8;
    pub const LD_HL_R: u8 = 8;
    pub const LD_HL_N: u8 = 12;
    pub const LD_A_BC: u8 = 8;
    pub const LD_A_DE: u8 = 8;
    pub const LD_A_NN: u8 = 16;
    pub const LD_BC_A: u8 = 8;
    pub const LD_DE_A: u8 = 8;
    pub const LD_NN_A: u8 = 16;
    pub const LD_A_FF00_N: u8 = 12;
    pub const LD_FF00_N_A: u8 = 12;
    pub const LD_A_FF00_C: u8 = 8;
    pub const LD_FF00_C_A: u8 = 8;
    pub const LD_SP_HL: u8 = 8;
    pub const LD_HL_SP_N: u8 = 12;
    pub const LD_BC_NN: u8 = 12;
    pub const LD_DE_NN: u8 = 12;
    pub const LD_HL_NN: u8 = 12;
    pub const LD_SP_NN: u8 = 12;
    pub const PUSH_RR: u8 = 16;
    pub const POP_RR: u8 = 12;
    pub const ADD_A_R: u8 = 4;
    pub const ADD_A_N: u8 = 8;
    pub const ADD_A_HL: u8 = 8;
    pub const ADC_A_R: u8 = 4;
    pub const ADC_A_N: u8 = 8;
    pub const ADC_A_HL: u8 = 8;
    pub const SUB_A_R: u8 = 4;
    pub const SUB_A_N: u8 = 8;
    pub const SUB_A_HL: u8 = 8;
    pub const SBC_A_R: u8 = 4;
    pub const SBC_A_N: u8 = 8;
    pub const SBC_A_HL: u8 = 8;
    pub const AND_A_R: u8 = 4;
    pub const AND_A_N: u8 = 8;
    pub const AND_A_HL: u8 = 8;
    pub const XOR_A_R: u8 = 4;
    pub const XOR_A_N: u8 = 8;
    pub const XOR_A_HL: u8 = 8;
    pub const OR_A_R: u8 = 4;
    pub const OR_A_N: u8 = 8;
    pub const OR_A_HL: u8 = 8;
    pub const CP_A_R: u8 = 4;
    pub const CP_A_N: u8 = 8;
    pub const CP_A_HL: u8 = 8;
    pub const INC_R: u8 = 4;
    pub const INC_HL: u8 = 12;
    pub const DEC_R: u8 = 4;
    pub const DEC_HL: u8 = 12;
    pub const ADD_HL_RR: u8 = 8;
    pub const INC_RR: u8 = 8;
    pub const DEC_RR: u8 = 8;
    pub const JP_NN: u8 = 16;
    pub const JP_HL: u8 = 4;
    pub const JP_CC_NN: u8 = 12;
    pub const JR_N: u8 = 12;
    pub const JR_CC_N: u8 = 8;
    pub const CALL_NN: u8 = 24;
    pub const CALL_CC_NN: u8 = 12;
    pub const RET: u8 = 16;
    pub const RET_CC: u8 = 8;
    pub const RETI: u8 = 16;
    pub const RST_N: u8 = 16;
    pub const DAA: u8 = 4;
    pub const CPL: u8 = 4;
    pub const SCF: u8 = 4;
    pub const CCF: u8 = 4;
    pub const HALT: u8 = 4;
    pub const STOP: u8 = 4;
    pub const DI: u8 = 4;
    pub const EI: u8 = 4;
    pub const RLCA: u8 = 4;
    pub const RLA: u8 = 4;
    pub const RRCA: u8 = 4;
    pub const RRA: u8 = 4;
    pub const RLC_R: u8 = 8;
    pub const RLC_HL: u8 = 16;
    pub const RL_R: u8 = 8;
    pub const RL_HL: u8 = 16;
    pub const RRC_R: u8 = 8;
    pub const RRC_HL: u8 = 16;
    pub const RR_R: u8 = 8;
    pub const RR_HL: u8 = 16;
    pub const SLA_R: u8 = 8;
    pub const SLA_HL: u8 = 16;
    pub const SRA_R: u8 = 8;
    pub const SRA_HL: u8 = 16;
    pub const SRL_R: u8 = 8;
    pub const SRL_HL: u8 = 16;
    pub const BIT_B_R: u8 = 8;
    pub const BIT_B_HL: u8 = 12;
    pub const SET_B_R: u8 = 8;
    pub const SET_B_HL: u8 = 16;
    pub const RES_B_R: u8 = 8;
    pub const RES_B_HL: u8 = 16;
    pub const SWAP_R: u8 = 8;
    pub const SWAP_HL: u8 = 16;
}

pub type Flags = u8;
pub const ZERO_FLAG: u8 = 0b1000_0000;
pub const SUBTRACT_FLAG: u8 = 0b0100_0000;
pub const HALF_CARRY_FLAG: u8 = 0b0010_0000;
pub const CARRY_FLAG: u8 = 0b0001_0000;
