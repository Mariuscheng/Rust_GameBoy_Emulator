// memory_viewer.rs - 記憶體檢視工具
use crate::mmu::MMU;

pub fn dump_memory_region(mmu: &MMU, start_addr: u16, length: usize) -> String {
    let mut output = String::new();
    output.push_str(&format!("📡 記憶體區域 0x{:04X} - 0x{:04X}:\n", start_addr, start_addr + (length as u16) - 1));
    
    for row in 0..((length + 15) / 16) {
        let row_addr = start_addr + (row * 16) as u16;
        output.push_str(&format!("0x{:04X}: ", row_addr));
        
        for col in 0..16 {
            let addr = row_addr + col;
            if (addr as usize) < (start_addr as usize) + length {
                output.push_str(&format!("{:02X} ", mmu.read_byte(addr)));
            } else {
                output.push_str("   ");
            }
        }
        
        output.push_str(" | ");
        
        for col in 0..16 {
            let addr = row_addr + col;
            if (addr as usize) < (start_addr as usize) + length {
                let byte = mmu.read_byte(addr);
                // 只顯示可列印字符
                if byte >= 32 && byte <= 126 {
                    output.push(byte as char);
                } else {
                    output.push('.');
                }
            }
        }
        
        output.push('\n');
    }
    
    output
}

pub fn dump_stack(mmu: &MMU, sp: u16, depth: usize) -> String {
    let mut output = String::new();
    output.push_str(&format!("📚 堆疊內容 (SP=0x{:04X}, 深度={}):\n", sp, depth));
    
    for i in 0..depth {
        let addr = sp + (i * 2) as u16;
        let value = mmu.read_byte(addr) as u16 | ((mmu.read_byte(addr + 1) as u16) << 8);
        output.push_str(&format!("SP+{:04X}: 0x{:04X}\n", i * 2, value));
    }
    
    output
}
