use crate::core::cycles::CyclesType;

// 指令特定週期
pub const NOP: CyclesType = 4;
pub const LD_R_R: CyclesType = 4;
pub const LD_R_N: CyclesType = 8;
pub const LD_R_HL: CyclesType = 8;
pub const LD_HL_R: CyclesType = 8;
pub const LD_HL_N: CyclesType = 12;
pub const LD_A_BC: CyclesType = 8;
pub const LD_A_DE: CyclesType = 8;
pub const LD_A_NN: CyclesType = 16;
pub const LD_NN_A: CyclesType = 16;
pub const LD_A_FF00_N: CyclesType = 12;
pub const LD_FF00_N_A: CyclesType = 12;
pub const LD_A_FF00_C: CyclesType = 8;
pub const LD_FF00_C_A: CyclesType = 8;
pub const LDI_HL_A: CyclesType = 8;
pub const LDI_A_HL: CyclesType = 8;
pub const LDD_HL_A: CyclesType = 8;
pub const LDD_A_HL: CyclesType = 8;
pub const LD_RR_NN: CyclesType = 12;
pub const LD_SP_NN: CyclesType = 12;
pub const LD_HL_SP_N: CyclesType = 12;
pub const LD_SP_HL: CyclesType = 8;
