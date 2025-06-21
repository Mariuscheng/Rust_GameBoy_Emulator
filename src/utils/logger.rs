use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Logger {
    pub debug_enabled: bool,
    pub vram_enabled: bool,
    pub mmu_enabled: bool,
}

impl Logger {
    pub fn new() -> Self {
        // Ensure logs directory exists
        if !Path::new("logs").exists() {
            let _ = fs::create_dir("logs");
        }

        Logger {
            debug_enabled: true,
            vram_enabled: true,
            mmu_enabled: true,
        }
    }

    fn get_timestamp() -> String {
        let start = SystemTime::now();
        let since_epoch = start.duration_since(UNIX_EPOCH).unwrap();
        format!("[{}.{}]", since_epoch.as_secs(), since_epoch.subsec_nanos())
    }

    pub fn log_vram(&self, msg: &str) {
        if !self.vram_enabled {
            return;
        }
        self.log_to_file(
            "logs/vram.log",
            &format!("{} VRAM: {}", Self::get_timestamp(), msg),
        );
    }

    pub fn log_mmu(&self, msg: &str) {
        if !self.mmu_enabled {
            return;
        }
        self.log_to_file(
            "logs/mmu.log",
            &format!("{} MMU: {}", Self::get_timestamp(), msg),
        );
    }

    pub fn log_ppu(&self, msg: &str) {
        if !self.debug_enabled {
            return;
        }
        self.log_to_file(
            "logs/debug_ppu.log",
            &format!("{} PPU: {}", Self::get_timestamp(), msg),
        );
    }

    pub fn log_debug(&self, msg: &str) {
        if !self.debug_enabled {
            return;
        }
        self.log_to_file(
            "logs/debug.txt",
            &format!("{} DEBUG: {}", Self::get_timestamp(), msg),
        );
    }

    pub fn log_boot_animation(&self, msg: &str) {
        self.log_to_file(
            "logs/boot_animation.log",
            &format!("{} Boot Animation: {}", Self::get_timestamp(), msg),
        );
    }

    fn log_to_file(&self, path: &str, msg: &str) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{}", msg);
        }
    }

    pub fn dump_vram(&self, vram: &[u8], start: usize, len: usize) {
        if !self.vram_enabled {
            return;
        }
        let end = (start + len).min(vram.len());
        let mut dump = format!("VRAM dump [{:04X}-{:04X}]:", start, end - 1);
        for (i, &byte) in vram[start..end].iter().enumerate() {
            if i % 16 == 0 {
                dump.push_str(&format!("\n{:04X}:", start + i));
            }
            dump.push_str(&format!(" {:02X}", byte));
        }
        self.log_vram(&dump);
    }

    pub fn log_tile_data(&self, tile_data: &[u8], tile_index: usize) {
        if !self.vram_enabled {
            return;
        }
        let mut tile_str = format!("Tile {:02X} data:", tile_index);
        for i in 0..16 {
            if i % 2 == 0 {
                tile_str.push_str("\n");
            }
            tile_str.push_str(&format!(" {:02X}", tile_data[i]));
        }
        self.log_vram(&tile_str);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger::new()
    }
}
