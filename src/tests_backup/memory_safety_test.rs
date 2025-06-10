// 記憶體安全測試模組
use crate::cpu::CPU;
use crate::mmu::MMU;

#[test]
fn test_mmu_memory_bounds_read() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    // 測試正常記憶體存取
    let normal_value = cpu.mmu.read_byte(0xFF40); 
    assert_eq!(normal_value, 0x91);
    
    // 測試邊界值存取
    let boundary_value = cpu.mmu.read_byte(0xFFFF);
    assert_eq!(boundary_value, 0x00);
    
    println!("記憶體讀取邊界檢查測試通過");
}

#[test]
fn test_mmu_memory_bounds_write() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    // 測試正常記憶體寫入
    cpu.mmu.write_byte(0xFF47, 0xE4);
    let read_back = cpu.mmu.read_byte(0xFF47);
    assert_eq!(read_back, 0xE4);
    
    // 測試邊界值寫入
    cpu.mmu.write_byte(0xFFFF, 0x01);
    let ie_value = cpu.mmu.read_byte(0xFFFF);
    assert_eq!(ie_value, 0x01);
    
    println!("記憶體寫入邊界檢查測試通過");
}

#[test]
fn test_vram_access_safety() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    // 測試 VRAM 存取
    cpu.mmu.write_vram(0x0000, 0x42);
    let value = cpu.mmu.read_vram(0x0000);
    assert_eq!(value, 0x42);
    
    // 測試 VRAM 邊界
    cpu.mmu.write_vram(0x1FFF, 0x33);
    let boundary_value = cpu.mmu.read_vram(0x1FFF);
    assert_eq!(boundary_value, 0x33);
    
    println!("VRAM 存取安全測試通過");
}

#[test]
fn test_oam_access_safety() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    // 測試 OAM 存取
    cpu.mmu.write_byte(0xFE00, 0x55);
    cpu.mmu.write_byte(0xFE9F, 0x66);
    
    assert_eq!(cpu.mmu.read_byte(0xFE00), 0x55);
    assert_eq!(cpu.mmu.read_byte(0xFE9F), 0x66);
    
    println!("OAM 存取安全測試通過");
}

#[test]
fn test_rom_access_safety() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    // 載入測試 ROM
    let test_rom = vec![0x00, 0x01, 0x02, 0x03];
    cpu.load_rom(&test_rom);
    
    // 測試 ROM 讀取
    assert_eq!(cpu.mmu.read_byte(0x0000), 0x00);
    assert_eq!(cpu.mmu.read_byte(0x0003), 0x03);
    
    // 測試超出範圍
    let out_of_bounds = cpu.mmu.read_byte(0x0010);
    assert_eq!(out_of_bounds, 0xFF);
    
    println!("ROM 存取安全測試通過");
}

#[test]
fn test_memory_stress() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    // 載入測試 ROM
    let test_rom = vec![0x00; 0x8000];
    cpu.load_rom(&test_rom);
    
    // 執行大量記憶體存取
    for i in 0..100 {
        let addr = (i * 13) % 0x10000;
        
        match addr {
            0x8000..=0x9FFF => {
                cpu.mmu.write_byte(addr as u16, (i % 256) as u8);
                let _ = cpu.mmu.read_byte(addr as u16);
            }
            0xFE00..=0xFE9F => {
                cpu.mmu.write_byte(addr as u16, (i % 256) as u8);
                let _ = cpu.mmu.read_byte(addr as u16);
            }
            _ => {
                let _ = cpu.mmu.read_byte(addr as u16);
            }
        }
    }
    
    println!("記憶體壓力測試通過");
}

#[test]
fn test_memory_boundaries_comprehensive() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    
    let test_addresses = vec![
        0x0000, 0x7FFF, // ROM
        0x8000, 0x9FFF, // VRAM
        0xA000, 0xBFFF, // 外部 RAM
        0xC000, 0xDFFF, // 工作 RAM
        0xFE00, 0xFE9F, // OAM
        0xFF00, 0xFF7F, // I/O
        0xFF80, 0xFFFE, // 高速 RAM
        0xFFFF,         // IE
    ];
    
    for &addr in &test_addresses {
        let _value = cpu.mmu.read_byte(addr);
        cpu.mmu.write_byte(addr, 0x42);
    }
    
    println!("綜合記憶體邊界測試通過：測試了 {} 個地址", test_addresses.len());
}
