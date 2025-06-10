// 測試 opcodes F0 和 44 的實現
use crate::cpu::CPU;
use crate::mmu::MMU;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_opcode_f0_ldh_a_n() {
        // 測試 0xF0: LDH A, (n) - 從 0xFF00+n 載入到 A
        let mmu = MMU::new();
        let mut cpu = CPU::new(mmu); // 載入一個較大的測試 ROM (至少 0x150 字節)
        let mut test_rom = vec![0; 0x8000]; // 32KB ROM
        test_rom[0] = 0xF0; // LDH A, (n)
        test_rom[1] = 0x80; // 地址 0xFF80
        cpu.load_rom(&test_rom);

        // 設置測試數據：在 0xFF80 放入值 0x42
        cpu.mmu.write_byte(0xFF80, 0x42);

        // 設置 PC 並執行 LDH A, (0x80) 指令
        cpu.registers.pc = 0x0000;
        cpu.registers.a = 0x00; // 確保 A 初始值為 0

        // 執行指令
        cpu.execute();

        // 檢查結果：A 應該包含 0x42
        assert_eq!(cpu.registers.a, 0x42);
        assert_eq!(cpu.registers.pc, 0x0002); // PC 應該增加 2
    }
    #[test]
    fn test_opcode_44_ld_b_h() {
        // 測試 0x44: LD B, H - 將 H 載入 B
        let mmu = MMU::new();
        let mut cpu = CPU::new(mmu); // 載入一個較大的測試 ROM (至少 0x150 字節)
        let mut test_rom = vec![0; 0x8000]; // 32KB ROM
        test_rom[0] = 0x44; // LD B, H
        cpu.load_rom(&test_rom);

        // 設置測試數據
        cpu.registers.h = 0x55;
        cpu.registers.b = 0x00; // 確保 B 初始值不同

        // 設置 PC 並執行 LD B, H 指令
        cpu.registers.pc = 0x0000;

        // 執行指令
        cpu.execute();

        // 檢查結果：B 應該包含 H 的值
        assert_eq!(cpu.registers.b, 0x55);
        assert_eq!(cpu.registers.h, 0x55); // H 應該保持不變
        assert_eq!(cpu.registers.pc, 0x0001); // PC 應該增加 1
    }
    #[test]
    fn test_opcodes_integration() {
        // 測試兩個 opcodes 的組合使用
        let mmu = MMU::new();
        let mut cpu = CPU::new(mmu); // 載入一個較大的測試 ROM (至少 0x150 字節)
        let mut test_rom = vec![0; 0x8000]; // 32KB ROM
        test_rom[0] = 0x44; // LD B, H
        test_rom[1] = 0xF0; // LDH A, (n)
        test_rom[2] = 0x90; // 地址 0xFF90
        cpu.load_rom(&test_rom);

        // 設置測試場景：
        // 1. 使用 LD B, H 將 H 的值載入 B
        // 2. 使用 LDH A, (n) 從高記憶體載入值到 A

        cpu.registers.h = 0x88;
        cpu.mmu.write_byte(0xFF90, 0x77);

        // 第一個指令：LD B, H (0x44)
        cpu.registers.pc = 0x0000;
        cpu.execute();

        // 第二個指令：LDH A, (0x90) (0xF0 0x90)
        cpu.execute();

        // 檢查結果
        assert_eq!(cpu.registers.b, 0x88); // B 應該等於原來的 H
        assert_eq!(cpu.registers.a, 0x77); // A 應該等於從 0xFF90 載入的值
        assert_eq!(cpu.registers.pc, 0x0003); // PC 應該在第三個位置
    }
}
