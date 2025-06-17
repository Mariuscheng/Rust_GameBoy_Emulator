#![allow(unused_variables)]
#![allow(dead_code)]

use super::{
    background::BackgroundRenderer, display::Display, sprite::SpriteRenderer,
    window::WindowRenderer,
};
use crate::config::VideoConfig;
use crate::cpu::{interrupts::InterruptRegisters, LCDC_BIT, VBLANK_BIT};
use crate::mmu::MMU;
use crate::utils::Logger;
use crate::{error::Error, error::Result};
use log::info;
use std::{cell::RefCell, rc::Rc, time::Instant};

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

/// PPU 模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PPUMode {
    HBlank = 0,  // 水平空白期
    VBlank = 1,  // 垂直空白期
    OAMScan = 2, // 掃描 OAM
    Drawing = 3, // 繪製像素
}

/// Picture Processing Unit (PPU)
#[derive(Debug)]
pub struct PPU {
    mode: PPUMode,
    mode_clock: u32,
    mmu: Rc<RefCell<MMU>>,
    background_renderer: BackgroundRenderer,
    window_renderer: WindowRenderer,
    sprite_renderer: SpriteRenderer,
    display: Display,
    config: VideoConfig,
    logger: RefCell<Logger>,
    interrupt_registers: Option<Rc<RefCell<InterruptRegisters>>>,

    // LCD 控制寄存器
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    window_line: u8,

    last_update: Instant,
    frame_count: u32,
}

impl PPU {
    pub fn new(mmu: Rc<RefCell<MMU>>, config: VideoConfig) -> Self {
        let mut display = Display::new();
        display.init_config(config.clone());

        PPU {
            mode: PPUMode::OAMScan,
            mode_clock: 0,
            background_renderer: BackgroundRenderer::new(Rc::clone(&mmu)),
            window_renderer: WindowRenderer::new(Rc::clone(&mmu)),
            sprite_renderer: SpriteRenderer::new(Rc::clone(&mmu)),
            display,
            config,
            logger: RefCell::new(Logger::default()),
            interrupt_registers: None,
            mmu,

            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            window_line: 0,

            last_update: Instant::now(),
            frame_count: 0,
        }
    }

    /// 設定中斷寄存器
    pub fn set_interrupt_registers(&mut self, registers: Rc<RefCell<InterruptRegisters>>) {
        self.interrupt_registers = Some(registers);
    }

    /// 檢查 VRAM 是否可以存取
    pub fn is_vram_accessible(&self) -> bool {
        self.mode != PPUMode::Drawing
    }

    /// 檢查 OAM 是否可以存取
    pub fn is_oam_accessible(&self) -> bool {
        self.mode != PPUMode::OAMScan && self.mode != PPUMode::Drawing
    }

    /// 記錄錯誤訊息
    fn log_error(&self, message: &str) {
        if let Ok(mut logger) = self.logger.try_borrow_mut() {
            logger.error(message);
        }
    }

    /// 處理 PPU 錯誤
    fn handle_ppu_error<T>(&self, error: impl std::fmt::Display) -> Result<T> {
        let error_msg = format!("PPU 錯誤: {}", error);
        self.log_error(&error_msg);
        Err(Error::Memory(error_msg))
    }

    pub fn init_config(&mut self, config: VideoConfig) {
        self.display.init_config(config);
    }

    pub fn reset(&mut self) {
        self.mode = PPUMode::HBlank;
        self.mode_clock = 0;
        self.ly = 0;
        self.stat &= !0x03; // 清除模式位元
        self.stat |= self.mode as u8;
    }

    pub fn get_display_buffer(&self) -> Result<&[u8]> {
        Ok(self.display.get_buffer())
    }

    fn is_lcd_enabled(&self) -> bool {
        (self.lcdc & 0x80) != 0
    }

    fn is_window_enabled(&self) -> bool {
        (self.lcdc & 0x20) != 0
    }

    fn is_sprites_enabled(&self) -> bool {
        (self.lcdc & 0x02) != 0
    }

    fn set_mode(&mut self, mode: PPUMode) {
        self.mode = mode;
        // 更新 STAT 寄存器的模式位
        let mut stat = self.stat & 0xFC; // 清除最後兩位
        stat |= mode as u8;
        self.stat = stat;

        // 檢查是否需要觸發STAT中斷
        let trigger_interrupt = match mode {
            PPUMode::HBlank if (self.stat & 0x08) != 0 => true,
            PPUMode::VBlank if (self.stat & 0x10) != 0 => true,
            PPUMode::OAMScan if (self.stat & 0x20) != 0 => true,
            _ => false,
        };

        if trigger_interrupt {
            if let Some(int_reg) = &self.interrupt_registers {
                int_reg.borrow_mut().request_interrupt(LCDC_BIT);
            }
        }
    }
    fn check_lyc(&mut self) {
        let lyc_match = self.ly == self.lyc;
        if lyc_match {
            self.stat |= 0x04; // 設置 LYC = LY 標誌
            if (self.stat & 0x40) != 0 {
                // LYC=LY 中斷使能
                if let Some(int_reg) = &self.interrupt_registers {
                    int_reg.borrow_mut().request_interrupt(LCDC_BIT);
                }
            }
        } else {
            self.stat &= !0x04; // 清除 LYC = LY 標誌
        }
    }

    pub fn tick(&mut self) -> Result<bool> {
        if !self.is_lcd_enabled() {
            self.mode = PPUMode::HBlank;
            self.ly = 0;
            self.mode_clock = 0;
            return Ok(false);
        }

        self.mode_clock += 1;

        match self.mode {
            PPUMode::HBlank => {
                if self.mode_clock >= 204 {
                    self.mode_clock = 0;
                    self.ly += 1;
                    self.check_lyc();

                    if self.ly >= 144 {
                        self.set_mode(PPUMode::VBlank);
                        if let Some(registers) = &self.interrupt_registers {
                            registers.borrow_mut().request_interrupt(VBLANK_BIT);
                        }
                        return Ok(true); // 完成一幀
                    } else {
                        self.set_mode(PPUMode::OAMScan);
                    }
                }
            }
            PPUMode::VBlank => {
                if self.mode_clock >= 456 {
                    self.mode_clock = 0;
                    self.ly += 1;
                    self.check_lyc();

                    if self.ly > 153 {
                        self.set_mode(PPUMode::OAMScan);
                        self.ly = 0;
                    }
                }
            }
            PPUMode::OAMScan => {
                if self.mode_clock >= 80 {
                    self.set_mode(PPUMode::Drawing);
                    self.mode_clock = 0;
                }
            }
            PPUMode::Drawing => {
                let sprite_count = match self.sprite_renderer.get_visible_sprite_count(self.ly) {
                    Ok(count) => count.min(10),
                    Err(_) => 0, // Consider logging this error
                };
                // Approximate drawing cycles, can be more precise based on sprite interactions
                let drawing_cycles = 172u32; // Simplified, actual can be 172-289 based on sprites

                if self.mode_clock >= drawing_cycles {
                    self.set_mode(PPUMode::HBlank);
                    self.mode_clock = 0;
                    // Log LCDC before rendering the scanline
                    let current_lcdc = self.mmu.borrow().read_byte(0xFF40).unwrap_or_else(|e| {
                        if let Ok(mut logger) = self.logger.try_borrow_mut() {
                            logger.error(&format!("Error reading LCDC for logging: {}", e));
                        }
                        self.lcdc // Fallback to stored LCDC if read fails
                    });

                    if let Ok(mut logger) = self.logger.try_borrow_mut() {
                        logger.debug(&format!(
                            "[PPU Tick] LCDC: {:02X} before rendering LY: {}",
                            current_lcdc, self.ly
                        ));
                    }

                    if let Err(e) = self.render_scan_line() {
                        println!("錯誤：渲染掃描線失敗: {}", e);
                        // Consider how to handle render errors, e.g., skip frame or panic
                    }
                }
            }
        }

        Ok(false) // 尚未完成這一幀
    }

    fn update_stat(&mut self) {
        let mut stat = self.stat & 0xFC;
        stat |= self.mode as u8;

        if self.ly == self.lyc {
            stat |= 0x04;
            if (stat & 0x40) != 0 {
                if let Some(int_reg) = &self.interrupt_registers {
                    int_reg.borrow_mut().request_interrupt(LCDC_BIT);
                }
            }
        }

        self.stat = stat;
    }

    fn get_color(&self, color_id: u8, palette: u8) -> [u8; 4] {
        let masked_color_id = color_id & 0b11;
        let shift = masked_color_id * 2;
        let color = (palette >> shift) & 0b11;

        // 僅在調試模式下輸出日誌
        if cfg!(debug_assertions) {
            info!(
                "顏色處理: ID={}, 調色板={:#04x}, 結果={}",
                masked_color_id, palette, color
            );
        }

        // 使用 GameBoy 原始調色板顏色
        match color {
            0 => [255, 255, 255, 255], // White
            1 => [170, 170, 170, 255], // Light Gray
            2 => [85, 85, 85, 255],    // Dark Gray
            3 => [0, 0, 0, 255],       // Black
            _ => unreachable!(),       // 理論上不可能發生
        }
    }

    pub fn update_lcdc(&mut self, value: u8) {
        self.lcdc = value;
        println!("[PPU] LCDC updated to: {:02X}", value);
        // Potentially reset PPU state or re-evaluate rendering based on new LCDC bits
        if !self.is_lcd_enabled() {
            // If LCD is turned off, PPU might need to clear screen or stop rendering
            println!("[PPU] LCD Disabled via LCDC write");
            self.display.clear(); // Example: clear display buffer
            self.ly = 0; // Reset LY
            self.set_mode(PPUMode::HBlank); // Set to a safe mode
                                            // Further state resets might be needed
        }
    }

    pub fn update_bgp(&mut self, value: u8) {
        self.bgp = value;
        self.display.bgp = value; // Also update the display's copy if it has one        println!("[PPU] BGP updated to: {:02X}", value);
    }

    fn render_scan_line(&mut self) -> Result<()> {
        let mut line_buffer = vec![[255, 255, 255, 255]; SCREEN_WIDTH];

        // 渲染背景
        if self.lcdc & 0x01 != 0 {
            match self.background_renderer.render_scanline(self.ly) {
                Ok(bg_buffer) => line_buffer = bg_buffer,
                Err(e) => return self.handle_ppu_error(e),
            }
        }

        // 渲染窗口
        if self.lcdc & 0x20 != 0 && self.wy <= self.ly {
            if let Err(e) =
                self.window_renderer
                    .render_scan_line(self.ly, &mut line_buffer, self.window_line)
            {
                return self.handle_ppu_error(e);
            }
        }

        // 渲染精靈
        if self.lcdc & 0x02 != 0 {
            if let Err(e) = self.sprite_renderer.render_scan_line(
                self.ly,
                &mut line_buffer,
                self.obp0,
                self.obp1,
                (self.lcdc & 0x04) != 0,
            ) {
                return self.handle_ppu_error(e);
            }
        }

        // 更新顯示
        if let Err(e) = self.display.update_line(self.ly as usize, &line_buffer) {
            return self.handle_ppu_error(e);
        }

        if self.ly >= 144 && self.window_line > 0 {
            self.window_line = 0;
        }

        Ok(())
    }
}
