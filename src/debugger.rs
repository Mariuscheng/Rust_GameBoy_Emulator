use std::fs::File;
use std::io::Write;
use chrono::Local;

pub struct Debugger {
    log_file: Option<File>,
    enabled: bool,
}

impl Debugger {
    pub fn new(enabled: bool) -> Self {
        let log_file = if enabled {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let log_path = format!("logs/debug_{}.log", timestamp);
            std::fs::create_dir_all("logs").ok();
            Some(File::create(log_path).unwrap_or_else(|_| panic!("Failed to create debug log file")))
        } else {
            None
        };

        Self {
            log_file,
            enabled,
        }
    }

    pub fn log(&mut self, message: &str) {
        if self.enabled {
            if let Some(file) = &mut self.log_file {
                let timestamp = Local::now().format("%H:%M:%S%.3f");
                writeln!(file, "[{}] {}", timestamp, message).unwrap_or_else(|e| eprintln!("Failed to write to debug log: {}", e));
            }
        }
    }

    pub fn log_cpu(&mut self, pc: u16, opcode: u8, registers: &str) {
        self.log(&format!("CPU - PC: {:04X}, OP: {:02X}, Registers: {}", pc, opcode, registers));
    }

    pub fn log_ppu(&mut self, message: &str) {
        self.log(&format!("PPU - {}", message));
    }

    pub fn log_mmu(&mut self, address: u16, value: u8, is_write: bool) {
        let operation = if is_write { "Write" } else { "Read" };
        self.log(&format!("MMU - {}: {:04X} = {:02X}", operation, address, value));
    }

    pub fn log_vram(&mut self, address: u16, value: u8) {
        self.log(&format!("VRAM - Write: {:04X} = {:02X}", address, value));
    }

    pub fn dump_vram(&mut self, vram: &[u8]) {
        if self.enabled {
            self.log("VRAM Dump:");
            for (i, chunk) in vram.chunks(16).enumerate() {
                let hex = chunk.iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.log(&format!("{:04X}: {}", i * 16, hex));
            }
        }
    }
}
