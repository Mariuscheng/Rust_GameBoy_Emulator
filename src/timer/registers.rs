// 計時器的常量定義
pub const DIV_REGISTER: u16 = 0xFF04;
pub const TIMA_REGISTER: u16 = 0xFF05;
pub const TMA_REGISTER: u16 = 0xFF06;
pub const TAC_REGISTER: u16 = 0xFF07;

// TAC 寄存器的位元定義
pub const TAC_ENABLE: u8 = 1 << 2;
pub const TAC_CLOCK_SELECT: u8 = 0b11;

// 計時器的不同頻率
pub const CLOCK_FREQUENCIES: [u32; 4] = [
    4096,   // 00: 4096 Hz    (CPU Clock / 1024)
    262144, // 01: 262144 Hz  (CPU Clock / 16)
    65536,  // 10: 65536 Hz   (CPU Clock / 64)
    16384,  // 11: 16384 Hz   (CPU Clock / 256)
];
