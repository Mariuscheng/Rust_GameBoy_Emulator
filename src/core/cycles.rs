/// CPU 和其他硬體組件共用的時脈週期型別
pub type CyclesType = u32;

/// CPU 時脈週期常數
pub const CYCLES_1: CyclesType = 4;    // 1 M-cycle = 4 T-cycles
pub const CYCLES_2: CyclesType = 8;    // 2 M-cycles = 8 T-cycles
pub const CYCLES_3: CyclesType = 12;   // 3 M-cycles = 12 T-cycles
pub const CYCLES_4: CyclesType = 16;   // 4 M-cycles = 16 T-cycles
pub const CYCLES_5: CyclesType = 20;   // 5 M-cycles = 20 T-cycles
pub const CYCLES_6: CyclesType = 24;   // 6 M-cycles = 24 T-cycles

/// CPU 與 PPU 同步基準週期
pub const CPU_CLOCK: CyclesType = 4_194_304;  // 4.194304 MHz
pub const PPU_LINE_CYCLES: CyclesType = 456;   // 掃描線週期數
pub const PPU_FRAME_CYCLES: CyclesType = 70224; // 幀週期數
