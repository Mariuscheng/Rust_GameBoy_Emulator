use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 日誌記錄器
#[derive(Debug)]
pub struct Logger {
    debug_log: File,
}

impl Logger {
    /// 創建新的日誌記錄器
    pub fn new() -> Self {
        // 確保logs目錄存在
        let logs_dir = Path::new("logs");
        if !logs_dir.exists() {
            fs::create_dir(logs_dir).unwrap_or_else(|e| {
                eprintln!("無法創建 logs 目錄: {}", e);
            });
        }

        // 使用當前時間作為日誌檔名
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let filename = format!("logs/debug_{}.log", now.as_secs());

        let debug_log = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&filename)
            .unwrap_or_else(|e| {
                panic!("無法打開日誌檔案 {}: {}", filename, e);
            });

        Logger { debug_log }
    }

    /// 記錄調試資訊
    pub fn debug(&mut self, message: &str) {
        let now = Local::now();
        if let Err(e) = writeln!(
            self.debug_log,
            "[{} DEBUG] {}",
            now.format("%Y-%m-%d %H:%M:%S"),
            message
        ) {
            eprintln!("無法寫入日誌: {}", e);
            return;
        }

        // 強制寫入到磁碟
        if let Err(e) = self.debug_log.flush() {
            eprintln!("無法刷新日誌: {}", e);
        }
    }

    /// 記錄錯誤資訊
    pub fn error(&mut self, message: &str) {
        let now = Local::now();
        writeln!(
            self.debug_log,
            "[{} ERROR] {}",
            now.format("%Y-%m-%d %H:%M:%S"),
            message
        )
        .unwrap_or_else(|e| {
            eprintln!("無法寫入日誌: {}", e);
        });
    }

    /// 記錄資訊
    pub fn info(&mut self, message: &str) {
        let now = Local::now();
        writeln!(
            self.debug_log,
            "[{} INFO] {}",
            now.format("%Y-%m-%d %H:%M:%S"),
            message
        )
        .unwrap_or_else(|e| {
            eprintln!("無法寫入日誌: {}", e);
        });
    }

    /// 記錄警告資訊
    pub fn warn(&mut self, message: &str) {
        let now = Local::now();
        writeln!(
            self.debug_log,
            "[{} WARN] {}",
            now.format("%Y-%m-%d %H:%M:%S"),
            message
        )
        .unwrap_or_else(|e| {
            eprintln!("無法寫入日誌: {}", e);
        });
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger::new()
    }
}
