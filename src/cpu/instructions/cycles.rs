// CPU 時脈週期常數
pub const CYCLES_1: u8 = 4;   // 1 M-cycle = 4 T-cycles
pub const CYCLES_2: u8 = 8;   // 2 M-cycles = 8 T-cycles
pub const CYCLES_3: u8 = 12;  // 3 M-cycles = 12 T-cycles
pub const CYCLES_4: u8 = 16;  // 4 M-cycles = 16 T-cycles
pub const CYCLES_5: u8 = 20;  // 5 M-cycles = 20 T-cycles
pub const CYCLES_6: u8 = 24;  // 6 M-cycles = 24 T-cycles

// 指令特定週期
pub const NOP: u8 = CYCLES_1;
pub const LD_R_R: u8 = CYCLES_1;
pub const LD_R_N: u8 = CYCLES_2;
pub const LD_R_HL: u8 = CYCLES_2;
pub const LD_HL_R: u8 = CYCLES_2;
pub const LD_HL_N: u8 = CYCLES_3;
pub const LD_A_BC: u8 = CYCLES_2;
pub const LD_A_DE: u8 = CYCLES_2;
pub const LD_A_NN: u8 = CYCLES_4;
pub const LD_NN_A: u8 = CYCLES_4;
pub const LD_A_FF00_N: u8 = CYCLES_3;
pub const LD_FF00_N_A: u8 = CYCLES_3;
pub const LD_A_FF00_C: u8 = CYCLES_2;
pub const LD_FF00_C_A: u8 = CYCLES_2;
pub const LDI_HL_A: u8 = CYCLES_2;
pub const LDI_A_HL: u8 = CYCLES_2;
pub const LDD_HL_A: u8 = CYCLES_2;
pub const LDD_A_HL: u8 = CYCLES_2;
pub const LD_RR_NN: u8 = CYCLES_3;
pub const LD_SP_NN: u8 = CYCLES_3;
pub const LD_HL_SP_N: u8 = CYCLES_3;
pub const LD_SP_HL: u8 = CYCLES_2;
