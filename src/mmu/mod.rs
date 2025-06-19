pub mod lcd_registers;
pub mod mbc;

use crate::cpu::interrupts::InterruptRegisters;
use crate::error::{Error, Result};
use crate::joypad::Joypad;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

// Game Boy 記憶體映射
// 0000-3FFF: 16KB ROM Bank 00（卡匣）
// 4000-7FFF: 16KB ROM Bank 01..NN（卡匣）
// 8000-9FFF: 8KB 視訊 RAM (VRAM)
// A000-BFFF: 8KB 外部 RAM（卡匣）
// C000-CFFF: 4KB 工作 RAM Bank 0
// D000-DFFF: 4KB 工作 RAM Bank 1-7
// E000-FDFF: Echo RAM（C000-DDFF 的鏡像）
// FE00-FE9F: Sprite 屬性表 (OAM)
// FEA0-FEFF: 未使用
// FF00-FF7F: I/O 寄存器
// FF80-FFFE: 高速 RAM (HRAM)
// FFFF-FFFF: 中斷啟用寄存器 (IE)

// #[derive(Debug)] // ⚠️ 移除 Debug 衍生，因為 Fn trait 不支援 Debug
pub struct MMU {
    pub memory: Vec<u8>, // 完整記憶體空間
    rom: Vec<u8>,        // 卡匣 ROM
    rom_bank: usize,     // 當前 ROM 庫編號
    ram_bank: usize,     // 當前 RAM 庫編號
    ram_enabled: bool,   // RAM 是否啟用
    mbc_type: u8,        // MBC 類型
    // boot_rom_enabled: bool,                                       // 啟動 ROM 是否啟用
    joypad: Option<Rc<RefCell<Joypad>>>, // 鍵盤控制
    interrupt_registers: Option<Rc<RefCell<InterruptRegisters>>>, // 中斷寄存器
    dma_transfer: bool,                  // 是否正在進行 DMA 傳輸
    dma_source: u16,                     // DMA 源地址
    dma_remaining: u8,                   // DMA 剩餘位元組
    vram: RefCell<Vec<u8>>,
    // 新增：PPU mode 查詢 callback
    #[allow(dead_code)]
    ppu_mode_getter: Option<Box<dyn Fn() -> u8>>,
    // 新增：PPU LY 查詢 callback
    #[allow(dead_code)]
    ppu_ly_getter: Option<Box<dyn Fn() -> u8>>,
}

impl MMU {
    pub fn new() -> Self {
        // 初始化完整的記憶體空間，64KB
        let memory = vec![0; 0x10000]; // 創建帶有預初始化 VRAM 的 MMU 實例
        let mmu = Self {
            memory,
            rom: Vec::new(),
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            mbc_type: 0,
            // boot_rom_enabled: false,
            joypad: None,
            interrupt_registers: None,
            dma_transfer: false,
            dma_source: 0,
            dma_remaining: 0,
            vram: RefCell::new(vec![0; 8 * 1024]), // 直接初始化 8KB VRAM
            ppu_mode_getter: None,                 // 預設無 callback
            ppu_ly_getter: None,
        };

        // log: MMU 初始化完成，VRAM 大小: {} bytes
        mmu
    }

    /// 設定 PPU mode 查詢 callback
    pub fn set_ppu_mode_getter<F: 'static + Fn() -> u8>(&mut self, getter: F) {
        self.ppu_mode_getter = Some(Box::new(getter));
    }

    /// 設定 PPU LY 查詢 callback
    pub fn set_ppu_ly_getter<F: 'static + Fn() -> u8>(&mut self, getter: F) {
        self.ppu_ly_getter = Some(Box::new(getter));
    }

    pub fn load_rom(&mut self, data: Vec<u8>) {
        // 保存 ROM 數據
        self.rom = data.clone();

        // 檢測 MBC 類型
        if self.rom.len() > 0x147 {
            self.mbc_type = self.rom[0x147];
            // log: ROM類型: 0x{:02X}
        }

        // 將 ROM 拷貝到記憶體
        let len = std::cmp::min(self.rom.len(), 0x8000);
        for i in 0..len {
            self.memory[i] = self.rom[i];
        }

        // 初始化其他區域
        // VRAM
        for i in 0x8000..0xA000 {
            self.memory[i] = 0;
        }

        // 輸出 ROM 標題
        if self.rom.len() > 0x143 {
            let title_end = (0x134..=0x143)
                .find(|&i| i >= self.rom.len() || self.rom[i] == 0)
                .unwrap_or(0x144);

            let title_bytes = &self.rom[0x134..title_end];
            let _title = String::from_utf8_lossy(title_bytes);
            // log: ROM 標題: {}
        }
    }

    // 分離 ROM 存取
    fn handle_rom_access(&self, addr: u16) -> Result<u8> {
        if addr as usize >= self.rom.len() {
            Ok(0xFF)
        } else {
            Ok(self.rom[addr as usize])
        }
    }

    // 分離 VRAM 存取
    fn handle_vram_access(&self, addr: u16) -> Result<u8> {
        self.read_vram(addr)
    }

    // 分離 RAM 存取
    fn handle_ram_access(&self, addr: u16) -> Result<u8> {
        if addr as usize >= self.memory.len() {
            Err(Error::InvalidAddress(addr))
        } else {
            Ok(self.memory[addr as usize])
        }
    }

    // 分離 I/O 存取
    fn handle_io_access(&self, addr: u16) -> Result<u8> {
        match addr {
            0xFF00 => {
                if let Some(joypad) = &self.joypad {
                    Ok(joypad.borrow().get_state())
                } else {
                    Ok(0xFF)
                }
            }
            _ => self.handle_ram_access(addr),
        }
    }

    pub fn read_byte(&self, addr: u16) -> Result<u8> {
        let result = match addr {
            0xFF44 => {
                // 讀取 PPU 狀態的 ly
                if let Some(ppu_ly_getter) = &self.ppu_ly_getter {
                    Ok(ppu_ly_getter())
                } else {
                    Ok(0)
                }
            },
            0x0000..=0x3FFF => self.handle_rom_access(addr),
            0x4000..=0x7FFF => {
                let bank_addr = (self.rom_bank * 0x4000) + (addr as usize - 0x4000);
                if bank_addr >= self.rom.len() {
                    Ok(0xFF)
                } else {
                    Ok(self.rom[bank_addr])
                }
            }
            0x8000..=0x9FFF => self.handle_vram_access(addr),
            0xA000..=0xBFFF => self.handle_ram_access(addr),
            0xC000..=0xFDFF => self.handle_ram_access(addr),
            0xFF00..=0xFF7F => self.handle_io_access(addr),
            0xFF80..=0xFFFF => self.handle_ram_access(addr),
            _ => Ok(0xFF),
        };
        if (0xFF00..=0xFFFF).contains(&addr) {
            if let Ok(value) = result {
                if let Ok(mut dbg) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                    let _ = writeln!(dbg, "[IO-DBG] read_byte: addr=0x{:04X}, value=0x{:02X}", addr, value);
                }
            }
        }
        result
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) -> Result<()> {
        let mut log_file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/mmu_write.log") {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[MMU] Failed to open log file: {}", e);
                return Err(crate::error::Error::IO(e));
            }
        };
        use std::io::Write;
        if let Err(e) = writeln!(
            log_file,
            "write_byte: addr=0x{:04X}, value=0x{:02X}",
            addr, value
        ) {
            eprintln!("[MMU] Failed to write log: {}", e);
            return Err(crate::error::Error::IO(e));
        }
        match addr {
            0x0000..=0x7FFF => self.handle_rom_write(addr, value),
            0x8000..=0x9FFF => self.handle_vram_write(addr, value),
            0xA000..=0xBFFF => self.handle_ram_write(addr, value),
            0xC000..=0xFDFF => self.handle_ram_write(addr, value),
            0xFF00..=0xFF7F => self.handle_io_write(addr, value),
            0xFF80..=0xFFFF => self.handle_ram_write(addr, value),
            _ => self.handle_ram_write(addr, value),
        }?;

        // 強制同步 VRAM 寫入到 debug.txt
        if (0x8000..=0x9FFF).contains(&addr) {
            if let Ok(mut dbg) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                let _ = writeln!(dbg, "[VRAM-DBG] write_byte: addr=0x{:04X}, value=0x{:02X}", addr, value);
            }
        }

        // DEBUG: 追蹤 LY (0xFF44) 寫入
        if addr == 0xFF44 {
            if let Ok(mut dbg) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                let _ = writeln!(dbg, "[LY-DBG] write_byte: addr=0xFF44, value=0x{:02X}", value);
            }
        }

        // DEBUG: 追蹤 IF/IE 寫入
        if addr == 0xFF0F || addr == 0xFFFF {
            if let Ok(mut dbg) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                let _ = writeln!(dbg, "[INT-DBG] write_byte: addr=0x{:04X}, value=0x{:02X}", addr, value);
            }
        }

        Ok(())
    }

    // 分離 ROM 寫入（通常為 MBC 控制）
    fn handle_rom_write(&mut self, addr: u16, value: u8) -> Result<()> {
        self.handle_mbc_control(addr, value);
        Ok(())
    }

    // 分離 VRAM 寫入
    fn handle_vram_write(&mut self, addr: u16, value: u8) -> Result<()> {
        self.write_vram(addr, value)
    }

    // 分離 RAM 寫入
    fn handle_ram_write(&mut self, addr: u16, value: u8) -> Result<()> {
        if addr as usize >= self.memory.len() {
            Err(Error::InvalidAddress(addr))
        } else {
            self.memory[addr as usize] = value;
            Ok(())
        }
    }

    // 分離 I/O 寫入
    fn handle_io_write(&mut self, addr: u16, value: u8) -> Result<()> {
        match addr {
            0xFF00 => {
                if let Some(joypad) = &self.joypad {
                    joypad.borrow_mut().set_state(value);
                }
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize] = value;
                }
                Ok(())
            }
            0xFF46 => {
                self.start_dma_transfer(value);
                Ok(())
            }
            0xFF0F => {
                if let Some(interrupt_regs) = &self.interrupt_registers {
                    interrupt_regs.borrow_mut().if_reg = value;
                }
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize] = value;
                }
                Ok(())
            }
            0xFFFF => {
                if let Some(interrupt_regs) = &self.interrupt_registers {
                    interrupt_regs.borrow_mut().ie = value;
                }
                if (addr as usize) < self.memory.len() {
                    self.memory[addr as usize] = value;
                }
                Ok(())
            }
            _ => self.handle_ram_write(addr, value),
        }
    }

    pub fn read_byte_direct(&self, addr: u16) -> u8 {
        if (addr as usize) < self.memory.len() {
            self.memory[addr as usize]
        } else {
            0xFF // 超出範圍時返回 0xFF
        }
    }

    pub fn write_byte_direct(&mut self, addr: u16, value: u8) {
        if (addr as usize) < self.memory.len() {
            self.memory[addr as usize] = value;
        }
    }

    pub fn read_word(&self, addr: u16) -> Result<u16> {
        let lo = self.read_byte(addr)?;
        let hi = self.read_byte(addr.wrapping_add(1))?;
        Ok(((hi as u16) << 8) | (lo as u16))
    }

    pub fn write_word(&mut self, addr: u16, value: u16) -> Result<()> {
        let lo = (value & 0xFF) as u8;
        let hi = (value >> 8) as u8;
        self.write_byte(addr, lo)?;
        self.write_byte(addr.wrapping_add(1), hi)
    }

    fn handle_mbc_control(&mut self, addr: u16, value: u8) {
        match self.mbc_type {
            0x00 => {
                // ROM ONLY，不執行任何操作
            }
            0x01..=0x03 => {
                // MBC1
                match addr {
                    0x0000..=0x1FFF => {
                        // RAM 啟用
                        self.ram_enabled = (value & 0x0F) == 0x0A;
                    }
                    0x2000..=0x3FFF => {
                        // ROM 庫號選擇
                        let bank = value & 0x1F;
                        let bank = if bank == 0 { 1 } else { bank as usize };
                        self.rom_bank = (self.rom_bank & 0x60) | bank;
                    }
                    0x4000..=0x5FFF => {
                        // RAM 庫號或上部 ROM 庫號選擇
                        if addr <= 0x5FFF {
                            // 高位 ROM 庫號
                            self.rom_bank = (self.rom_bank & 0x1F) | ((value as usize & 0x03) << 5);
                        } else {
                            // RAM 庫號
                            self.ram_bank = value as usize & 0x03;
                        }
                    }
                    0x6000..=0x7FFF => {
                        // 模式選擇
                        // 暫時不實現
                    }
                    _ => {}
                }
            }
            _ => {
                // 其他 MBC 類型暫時不支援
            }
        }
    }

    fn start_dma_transfer(&mut self, value: u8) {
        // DMA 開始值是高位元組，表示來源地址的高8位
        let source_addr = (value as u16) << 8;
        self.dma_source = source_addr;
        self.dma_remaining = 0xA0; // 需要傳輸 160 位元組
        self.dma_transfer = true;

        // 立即完成 DMA 傳輸
        self.complete_dma();
    }

    fn complete_dma(&mut self) {
        // 完成 OAM DMA 傳輸
        if !self.dma_transfer {
            return;
        }

        // 從源地址複製到 OAM
        for i in 0..0xA0 {
            let source = self.dma_source + i;
            let dest = 0xFE00 + i;

            // 讀取源地址
            let value = match self.read_byte(source) {
                Ok(v) => v,
                Err(_) => 0xFF,
            };

            // 寫入 OAM
            if (dest as usize) < self.memory.len() {
                self.memory[dest as usize] = value;
            }
        }

        self.dma_transfer = false;
        self.dma_remaining = 0;
    }

    pub fn set_joypad(&mut self, joypad: Rc<RefCell<Joypad>>) {
        self.joypad = Some(joypad);
    }

    pub fn set_interrupt_registers(
        &mut self,
        interrupt_registers: Rc<RefCell<InterruptRegisters>>,
    ) {
        self.interrupt_registers = Some(interrupt_registers);
    }
    fn write_vram(&mut self, addr: u16, value: u8) -> Result<()> {
        // 新增：PPU mode 3 禁止寫入
        if let Some(ref getter) = self.ppu_mode_getter {
            if getter() == 3 {
                // log: 阻擋 VRAM 寫入於 Mode 3
                return Ok(()); // 或 Err(...) 視需求
            }
        }
        let vram_addr = (addr - 0x8000) as usize;
        if vram_addr >= self.vram.borrow().len() {
            // 防止越界訪問
            return Err(Error::Memory(format!("VRAM 訪問越界: 0x{:04X}", addr)));
        }
        let mut log_file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/vram_write.log") {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[MMU] Failed to open vram log file: {}", e);
                // 也寫入 debug.txt 方便追蹤
                if let Ok(mut fallback) = std::fs::OpenOptions::new().create(true).append(true).open("logs/debug.txt") {
                    let _ = writeln!(fallback, "[MMU] Failed to open vram_write.log: {}", e);
                }
                return Err(crate::error::Error::IO(e));
            }
        };
        use std::io::Write;
        if let Err(e) = writeln!(log_file, "VRAM[0x{:04X}] = 0x{:02X}", addr, value) {
            eprintln!("[MMU] Failed to write vram log: {}", e);
            return Err(crate::error::Error::IO(e));
        }
        self.vram.borrow_mut()[vram_addr] = value;
        Ok(())
    }

    fn read_vram(&self, addr: u16) -> Result<u8> {
        let vram_addr = (addr - 0x8000) as usize;
        if vram_addr >= self.vram.borrow().len() {
            // 防止越界訪問
            return Err(Error::Memory(format!("VRAM 訪問越界: 0x{:04X}", addr)));
        }
        Ok(self.vram.borrow()[vram_addr])
    }

    // 安全初始化 VRAM
    pub fn initialize_vram(&mut self) {
        let mut vram = self.vram.borrow_mut();
        if vram.is_empty() {
            // 如果 VRAM 尚未初始化，創建 8KB VRAM
            *vram = vec![0; 8 * 1024];
            // log: VRAM 已初始化: {} bytes
        }
    }

    pub fn vram(&self) -> Vec<u8> {
        self.vram.borrow().clone()
    }
    pub fn oam(&self) -> [u8; 160] {
        let mut arr = [0u8; 160];
        arr.copy_from_slice(&self.memory[0xFE00..0xFEA0]);
        arr
    }
}
