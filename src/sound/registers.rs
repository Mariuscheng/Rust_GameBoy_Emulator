// 聲道 1 寄存器 (方波 + 掃描)
pub const NR10: u16 = 0xFF10; // 掃描
pub const NR11: u16 = 0xFF11; // 音長/波形佔空比
pub const NR12: u16 = 0xFF12; // 音量包絡
pub const NR13: u16 = 0xFF13; // 頻率低 8 位
pub const NR14: u16 = 0xFF14; // 頻率高 3 位 + 控制

// 聲道 2 寄存器 (方波)
pub const NR21: u16 = 0xFF16; // 音長/波形佔空比
pub const NR22: u16 = 0xFF17; // 音量包絡
pub const NR23: u16 = 0xFF18; // 頻率低 8 位
pub const NR24: u16 = 0xFF19; // 頻率高 3 位 + 控制

// 聲道 3 寄存器 (波形)
pub const NR30: u16 = 0xFF1A; // 開/關
pub const NR31: u16 = 0xFF1B; // 音長
pub const NR32: u16 = 0xFF1C; // 輸出等級
pub const NR33: u16 = 0xFF1D; // 頻率低 8 位
pub const NR34: u16 = 0xFF1E; // 頻率高 3 位 + 控制
pub const WAVE_PATTERN: u16 = 0xFF30; // 波形圖案 (0xFF30-0xFF3F)

// 聲道 4 寄存器 (噪音)
pub const NR41: u16 = 0xFF20; // 音長
pub const NR42: u16 = 0xFF21; // 音量包絡
pub const NR43: u16 = 0xFF22; // 多項式計數器
pub const NR44: u16 = 0xFF23; // 控制

// 聲音控制寄存器
pub const NR50: u16 = 0xFF24; // 主音量/VIN 選擇
pub const NR51: u16 = 0xFF25; // 聲道混音
pub const NR52: u16 = 0xFF26; // 聲音開/關

// 控制位元
pub const MASTER_ENABLE: u8 = 0x80; // NR52 的位 7
pub const LENGTH_ENABLE: u8 = 0x40; // NRx4 的位 6
pub const TRIGGER: u8 = 0x80; // NRx4 的位 7

// 音量包絡標誌
pub const ENV_UP: u8 = 0x08; // 向上調整
pub const ENV_DOWN: u8 = 0x00; // 向下調整
