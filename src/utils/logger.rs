#[derive(Debug)]
pub struct Logger {
    // ...existing code...
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            // ...初始化內容...
        }
    }
    pub fn info(&self, _msg: &str) {}
    pub fn warn(&self, _msg: &str) {}
    pub fn error(&self, _msg: &str) {}
    pub fn log_to_file(&self, msg: &str) {
        use std::fs::OpenOptions;
        use std::io::Write;
        let log_path = "logs/debug_ppu.log";
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "{}", msg);
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger::new()
    }
}
