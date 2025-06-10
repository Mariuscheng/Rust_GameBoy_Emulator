// 測試運行器 - 不需要圖形界面，用於測試基本功能
use crate::cpu::CPU;
use crate::mmu::MMU;
use crate::ppu::PPU;

pub fn run_test_simulation() -> String {
    println!("開始測試模擬...");

    // 創建 MMU 和 CPU
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new(); // 不載入任何 ROM，讓 MMU 使用 fallback ROM
                              // cpu.load_rom(&test_rom); // 注釋掉這行
    println!("使用 MMU 的 fallback ROM 進行測試...");

    let mut results = String::new();
    results.push_str("=== Game Boy 模擬器測試結果 ===\n\n"); // 同步 PPU 數據 (只在開始時做一次)
    let vram_data = cpu.mmu.vram();
    ppu.vram.copy_from_slice(&vram_data);
    ppu.set_bgp(cpu.mmu.read_byte(0xFF47));
    ppu.set_lcdc(cpu.mmu.read_byte(0xFF40));

    // 運行一些指令週期
    for step in 0..100 {
        cpu.step();

        // 每 10 步驟記錄一次狀態並同步PPU
        if step % 10 == 0 {
            let status = cpu.get_enhanced_status_report();
            results.push_str(&format!("步驟 {}: {}\n", step, status));

            // 定期同步 PPU 數據（減少頻率）
            let updated_vram = cpu.mmu.vram();
            ppu.vram.copy_from_slice(&updated_vram);
            ppu.step();
        }

        // 模擬硬體狀態更新
        cpu.simulate_hardware_state();
    }
    results.push_str("\n=== 最終狀態 ===\n");
    results.push_str(&cpu.get_enhanced_status_report());

    // 檢查 VRAM 狀態（簡化版）
    let vram_data = cpu.mmu.vram();
    let vram_non_zero = vram_data.iter().filter(|&&b| b != 0).count();
    results.push_str("\n=== VRAM 分析 ===\n");
    results.push_str(&format!(
        "VRAM 非零字節數: {} / {}\n",
        vram_non_zero,
        vram_data.len()
    ));

    // 顯示前16字節的VRAM內容
    results.push_str("VRAM 前16字節: ");
    for i in 0..16.min(vram_data.len()) {
        results.push_str(&format!("{:02X} ", vram_data[i]));
    }
    results.push_str("\n");

    results.push_str("\n=== 測試完成 ===\n");

    results
}
