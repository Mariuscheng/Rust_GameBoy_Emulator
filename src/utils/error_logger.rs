use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

lazy_static! {
    pub static ref ERROR_LOGGER: Mutex<ErrorLogger> = Mutex::new(ErrorLogger::new());
}

#[macro_export]
macro_rules! debug_log {
    ($module:expr, $($arg:tt)*) => ({
        if let Ok(mut logger) = $crate::util::error_logger::ERROR_LOGGER.lock() {
            writeln!(logger.get_debug_log(), "[{}] {}", $module, format!($($arg)*)).ok();
        }
    })
}

#[macro_export]
macro_rules! error_log {
    ($module:expr, $($arg:tt)*) => ({
        if let Ok(mut logger) = $crate::util::error_logger::ERROR_LOGGER.lock() {
            writeln!(logger.get_error_log(), "[{}] {}", $module, format!($($arg)*)).ok();
        }
    })
}

#[derive(Debug, Default)]
pub struct ErrorLogger {
    error_log: Option<File>,
    debug_log: Option<File>,
}

impl ErrorLogger {
    pub fn new() -> Self {
        let log_dir = "logs";
        let _ = fs::create_dir_all(log_dir);

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let error_path = format!("{}/error_{}.log", log_dir, timestamp);
        let debug_path = format!("{}/debug_{}.log", log_dir, timestamp);

        let error_log = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(error_path)
            .ok();

        let debug_log = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(debug_path)
            .ok();

        Self {
            error_log,
            debug_log,
        }
    }

    pub fn get_error_log(&mut self) -> &mut File {
        self.error_log.as_mut().expect("Cannot open error log file")
    }

    pub fn get_debug_log(&mut self) -> &mut File {
        self.debug_log.as_mut().expect("Cannot open debug log file")
    }

    pub fn log_error<T: std::fmt::Display>(&mut self, module: &str, error: T) {
        if let Some(ref mut file) = self.error_log {
            let now: DateTime<Local> = Local::now();
            writeln!(
                file,
                "[{}] [{}] {}",
                now.format("%Y-%m-%d %H:%M:%S"),
                module,
                error
            )
            .ok();
        }
    }

    pub fn log_debug<T: std::fmt::Display>(&mut self, module: &str, message: T) {
        if let Some(ref mut file) = self.debug_log {
            let now: DateTime<Local> = Local::now();
            writeln!(
                file,
                "[{}] [{}] {}",
                now.format("%Y-%m-%d %H:%M:%S"),
                module,
                message
            )
            .ok();
        }
    }

    pub fn log_cpu_state<T: std::fmt::Display>(&mut self, message: T) {
        self.log_debug("CPU", message);
    }

    pub fn log_ppu_state<T: std::fmt::Display>(&mut self, message: T) {
        self.log_debug("PPU", message);
    }

    pub fn log_interrupt<T: std::fmt::Display>(&mut self, message: T) {
        self.log_debug("INT", message);
    }

    pub fn log_state<T: std::fmt::Display>(&mut self, message: T) {
        if let Some(ref mut file) = self.debug_log {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%3f");
            writeln!(file, "[{}] {}", timestamp, message).ok();
            file.flush().ok();
        }
    }
}
