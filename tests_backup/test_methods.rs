// 測試文件以檢查 MMU 方法
use crate::mmu::MMU;

fn test_mmu_methods() {
    let mut mmu = MMU::new();

    // 測試這些方法是否存在
    mmu.step_apu();
    mmu.step_timer();
    mmu.step_joypad();
    mmu.step_serial();
    mmu.step_dma();

    println!("所有方法都存在！");
}

fn main() {
    test_mmu_methods();
}
