// filepath: gameboy_emulator/src/main.rs
use minifb::{Key, Window, WindowOptions};
mod mmu;
use crate::mmu::MMU;
mod cpu;
use crate::cpu::CPU;
mod ppu;
use crate::ppu::PPU;

fn main() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let mut ppu = PPU::new();

    // 假設我們有一個簡單的 ROM 數據
    //let rom_data: Vec<u8> = vec![0x00; 0x8000]; // 32KB 的空白 ROM
    cpu.load_rom(include_bytes!("../test.gb"));

    // 檢查 ROM 前 16 bytes
    // println!("ROM 前 16 bytes: {:02X?}", &cpu.mmu.rom[0..16]);

    // 創建一個窗口
    let mut window = Window::new("Game Boy Emulator", 160, 144, WindowOptions::default()).unwrap();

    let mut last_bgp = 0xFF;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        for _ in 0..5_000_000 {
            cpu.step();
            cpu.mmu.step();
        }

        // 2. 處理鍵盤輸入
        let keys = window.get_keys_pressed(minifb::KeyRepeat::Yes);
        let mut joypad = 0xFF;
        for key in keys {
            match key {
                Key::Z => { joypad &= !0x01; } // A
                Key::X => { joypad &= !0x02; } // B
                Key::Backspace => { joypad &= !0x04; } // Select
                Key::Enter => { joypad &= !0x08; } // Start
                Key::Right => { joypad &= !0x10; } // Right
                Key::Left => { joypad &= !0x20; } // Left
                Key::Up => { joypad &= !0x40; } // Up
                Key::Down => { joypad &= !0x80; } // Down
                _ => {}
            }
        }
        cpu.mmu.set_joypad(joypad); // 使用 set_joypad() 方法

        // 3. 同步 VRAM 與 OAM
        ppu.vram.copy_from_slice(&cpu.mmu.vram);
        let oam_slice = cpu.mmu.get_oam();
        ppu.oam.copy_from_slice(oam_slice);

        // 4. 同步 PPU 暫存器
        ppu.lcdc = cpu.mmu.read_reg(0xFF40);
        ppu.stat = cpu.mmu.read_reg(0xFF41);
        ppu.scy  = cpu.mmu.read_reg(0xFF42);
        ppu.scx  = cpu.mmu.read_reg(0xFF43);
        ppu.ly   = cpu.mmu.read_reg(0xFF44);
        ppu.lyc  = cpu.mmu.read_reg(0xFF45);
        ppu.bgp  = cpu.mmu.read_reg(0xFF47);
        ppu.obp0 = cpu.mmu.read_reg(0xFF48);
        ppu.obp1 = cpu.mmu.read_reg(0xFF49);
        ppu.wy   = cpu.mmu.read_reg(0xFF4A);
        ppu.wx   = cpu.mmu.read_reg(0xFF4B);

        // 5. 執行 PPU 掃描線
        for _ in 0..154 { ppu.step(); }

        // 6. 顯示畫面
        let buffer = ppu.get_framebuffer();
        window.update_with_buffer(buffer, 160, 144).unwrap();
        let bgp = cpu.mmu.read_reg(0xFF47);
        if bgp != last_bgp {
            //println!("BGP changed: {:02X}", bgp);
            last_bgp = bgp;
        }
        cpu.mmu.if_reg |= 0x01; // 設定 VBlank 中斷旗標
    }
    //println!("Tile 0x00: {:02X?}", &cpu.mmu.vram[0x0000..0x0010]);
    //println!("Tile 0x39: {:02X?}", &cpu.mmu.vram[0x39 * 16..0x39 * 16 + 16]);
    //println!("BGP: {:02X}", cpu.mmu.read_reg(0xFF47));
    println!("VRAM: {:02X?}", &cpu.mmu.vram[0..16]);
    println!("PC: {:04X} LCDC: {:02X} IF: {:02X} IE: {:02X} BGP: {:02X}", cpu.registers.pc, cpu.mmu.read_reg(0xFF40), cpu.mmu.if_reg, cpu.mmu.ie_reg, cpu.mmu.read_reg(0xFF47));
    // println!("tile_index={} shade={} bgp={:02X}", tile_index, shade, ppu.bgp);
    //println!("Tile 0x00: {:02X?}", &cpu.mmu.vram[0x0000..0x0010]);
    //println!("Tile map 0x9800~0x9810: {:02X?}", &cpu.mmu.vram[0x1800..0x1810]);
    //println!("PC: {:04X}", cpu.pc);
    //println!("{:02X?}", &cpu.mmu.vram[0..16]);
}