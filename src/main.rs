use minifb::{Key, Window, WindowOptions};
use std::fs;

mod cpu;
mod mmu;

fn main() {
    let mmu = mmu::MMU::new();
    let mut cpu = cpu::CPU::new(mmu);

    let rom_path = std::env::args().nth(1).unwrap_or("test.gb".to_string());
    let rom = fs::read(&rom_path).unwrap_or_else(|e| {
        eprintln!("無法讀取 {}: {}. 使用預設 ROM。", rom_path, e);
        vec![0; 0x150]
    });
    cpu.load_rom(&rom);

    let mut window = Window::new("Game Boy Emulator", 160, 144, WindowOptions::default())
        .unwrap_or_else(|e| panic!("視窗創建失敗: {}", e));
    let mut buffer: Vec<u32> = vec![0; 160 * 144];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        cpu.step();
        // 這裡可以根據 VRAM 內容產生畫面
        // 例如：buffer 填黑
        buffer.fill(0x000000);
        window.update_with_buffer(&buffer, 160, 144).unwrap();
    }
}

