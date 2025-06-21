use crate::error::Result;
use crate::mmu::MMU;
use pixels::Pixels;
use std::io::Write;
use std::{cell::RefCell, rc::Rc};

/// PPU (像素處理單元)負責 Game Boy 的圖形渲染
pub struct PPU {
    /// MMU 引用
    mmu: Rc<RefCell<MMU>>,

    /// 160x144 畫面緩衝區
    framebuffer: Vec<u8>,

    /// FF47 - BGP - 背景調色板數據
    pub bgp: u8,

    /// FF48 - OBP0 - 物件調色板 0 數據
    pub obp0: u8,

    /// FF49 - OBP1 - 物件調色板 1 數據
    pub obp1: u8,

    /// FF43 - SCX - 背景水平捲動位置 (0-255)
    pub scx: u8,

    /// FF42 - SCY - 背景垂直捲動位置 (0-255)
    pub scy: u8,

    /// FF4B - WX - 視窗 X 位置減 7 (0-166)
    pub wx: u8,

    /// FF4A - WY - 視窗 Y 位置 (0-143)
    pub wy: u8,

    /// FF40 - LCDC - LCD 控制寄存器
    pub lcdc: u8,

    /// 用於 FPS 計算的時間點
    last_frame_time: std::time::Instant,

    /// FPS 計數器
    fps_counter: u32,

    /// 目前 PPU 模式 (0-3)
    pub mode: u8,

    /// FF44 - LY - 目前掃描線 (0-153)
    pub ly: u8,

    /// FF45 - LYC - 掃描線比較值
    pub lyc: u8,

    /// FF41 - STAT - LCD 狀態寄存器
    pub stat: u8,

    /// 點時鐘計數器
    pub dots: u32,

    /// Sprite 屬性表 (40個物件 * 4位元組)
    oam: [u8; 160],
}

impl PPU {
    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")
            .unwrap();
        writeln!(file, "[PPU-INIT] PPU初始化開始").unwrap();

        let ppu = Self {
            mmu,
            framebuffer: vec![0; 160 * 144],
            bgp: 0,
            obp0: 0,
            obp1: 0,
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,
            oam: [0; 160],
            lcdc: 0,
            last_frame_time: std::time::Instant::now(),
            fps_counter: 0,
            mode: 2,
            ly: 0,
            lyc: 0,
            stat: 0,
            dots: 0,
        };

        writeln!(file, "[PPU-INIT] PPU初始化完成").unwrap();
        ppu
    }

    pub fn reset(&mut self) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")
            .unwrap();
        writeln!(file, "[PPU-RESET] PPU重置開始").unwrap();

        self.mode = 2;
        self.ly = 0;
        self.dots = 0;
        self.framebuffer.fill(0);

        writeln!(file, "[PPU-RESET] PPU重置完成").unwrap();
    }

    pub fn step(&mut self, cycles: u32) -> Result<()> {
        self.dots += cycles;

        match self.mode {
            2 => {
                // OAM Scan Mode (80 dots)
                if self.dots >= 80 {
                    self.dots -= 80;
                    self.mode = 3;

                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/debug.txt")?;
                    writeln!(file, "[PPU] 進入模式3 (像素傳輸), LY={}", self.ly)?;
                }
            }
            3 => {
                // Pixel Transfer Mode (172 dots)
                if self.dots >= 172 {
                    self.dots -= 172;
                    self.mode = 0;

                    self.render_scanline()?;

                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/debug.txt")?;
                    writeln!(file, "[PPU] 進入模式0 (H-Blank), LY={}", self.ly)?;
                }
            }
            0 => {
                // H-Blank Mode (204 dots)
                if self.dots >= 204 {
                    self.dots -= 204;
                    self.ly += 1;

                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/debug.txt")?;
                    if self.ly == 144 {
                        self.mode = 1;
                        writeln!(file, "[PPU] Entering V-Blank (mode 1)")?;
                        // Set V-Blank interrupt
                        let mut mmu = self.mmu.borrow_mut();
                        let if_reg = mmu.read_byte(0xFF0F)?;
                        mmu.write_byte(0xFF0F, if_reg | 0x01)?; // Set V-Blank interrupt (bit 0)

                        // Log V-Blank interrupt trigger
                        if let Ok(mut vblank_file) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("logs/vblank.log")
                        {
                            writeln!(
                                vblank_file,
                                "V-Blank interrupt triggered, IF=0x{:02X}",
                                if_reg | 0x01
                            )
                            .ok();
                        }
                    } else {
                        self.mode = 2;
                        writeln!(file, "[PPU] 開始新的掃描線 {}", self.ly)?;
                    }
                }
            }
            1 => {
                // V-Blank Mode (4560 dots total, 10 scanlines)
                if self.dots >= 456 {
                    self.dots -= 456;
                    self.ly += 1;

                    let mut file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/debug.txt")?;

                    if self.ly > 153 {
                        self.ly = 0;
                        self.mode = 2;
                        writeln!(file, "[PPU] V-Blank結束，回到模式2")?;
                    }
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    pub fn render_scanline(&mut self) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")?;

        writeln!(file, "[PPU] 渲染掃描線 {}, 模式={}", self.ly, self.mode)?;

        // 開始渲染
        if self.lcdc & 0x80 != 0 {
            // LCD 顯示開啟
            // 先取得並記錄 VRAM 狀態
            {
                let mmu = self.mmu.borrow();
                let vram = mmu.vram().borrow();
                let vram_stats = vram
                    .iter()
                    .enumerate()
                    .filter(|(_, &v)| v != 0)
                    .take(10)
                    .collect::<Vec<_>>();

                if !vram_stats.is_empty() {
                    writeln!(file, "[PPU] VRAM 非零值: {:?}", vram_stats)?;
                } else {
                    writeln!(file, "[PPU] 警告：VRAM 全為0")?;
                }
            }

            // 然後進行渲染
            if self.lcdc & 0x01 != 0 {
                // 背景啟用
                self.render_background()?;
            }
            if self.lcdc & 0x20 != 0 {
                // 視窗啟用
                self.render_window()?;
            }
            if self.lcdc & 0x02 != 0 {
                // 精靈啟用
                self.render_sprites()?;
            }
        }

        Ok(())
    }

    pub fn render_background(&mut self) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")?;

        // 從 MMU 讀取 VRAM
        let mmu = self.mmu.borrow();
        let vram = mmu.vram().borrow();

        // 檢查背景瓦片數據
        let tile_data = &vram[0..0x1800];
        let mut has_tiles = false;

        for (i, chunk) in tile_data.chunks(16).enumerate() {
            if chunk.iter().any(|&x| x != 0) {
                has_tiles = true;
                writeln!(file, "[PPU] 發現非空瓦片 #{}: {:?}", i, chunk)?;
            }
        }

        if !has_tiles {
            writeln!(file, "[PPU] 警告：所有背景瓦片數據為0")?;
        }

        Ok(())
    }

    pub fn render_window(&mut self) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")?;

        writeln!(file, "[PPU] 渲染視窗，WX={}, WY={}", self.wx, self.wy)?;
        Ok(())
    }

    pub fn render_sprites(&mut self) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")?;

        writeln!(
            file,
            "[PPU] 渲染精靈，OAM數量: {}",
            self.oam.iter().filter(|&&x| x != 0).count()
        )?;
        Ok(())
    }

    pub fn render_frame(&mut self, pixels: &mut Pixels) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/debug.txt")?;

        writeln!(file, "\n[PPU] === 開始渲染新的一幀 ===")?;
        writeln!(
            file,
            "LCDC={:#04x}, LY={}, 模式={}",
            self.lcdc, self.ly, self.mode
        )?;

        // 檢查 VRAM 狀態
        let mmu = self.mmu.borrow();
        let vram = mmu.vram().borrow();
        let vram_stats = vram
            .iter()
            .enumerate()
            .filter(|(_, &v)| v != 0)
            .take(10)
            .collect::<Vec<_>>();

        if !vram_stats.is_empty() {
            writeln!(file, "[PPU] VRAM 非零值: {:?}", vram_stats)?;
        }

        // 更新畫面
        let frame = pixels.frame_mut();
        for (i, pixel) in self.framebuffer.iter().enumerate() {
            let rgba = match pixel & 0x03 {
                0 => [0xFF, 0xFF, 0xFF, 0xFF], // 白
                1 => [0xAA, 0xAA, 0xAA, 0xFF], // 淺灰
                2 => [0x55, 0x55, 0x55, 0xFF], // 深灰
                3 => [0x00, 0x00, 0x00, 0xFF], // 黑
                _ => unreachable!(),
            };

            let base = i * 4;
            frame[base..base + 4].copy_from_slice(&rgba);
        }

        writeln!(file, "[PPU] 幀渲染完成")?;
        Ok(())
    }

    fn update_background(&mut self, mmu: &MMU) -> Result<()> {
        let vram = mmu.get_vram().borrow();
        // ...rest of the function
        Ok(())
    }

    fn update_window(&mut self, mmu: &MMU) -> Result<()> {
        let vram = mmu.get_vram().borrow();
        // ...rest of the function
        Ok(())
    }

    fn update_sprites(&mut self, mmu: &MMU) -> Result<()> {
        let vram = mmu.get_vram().borrow();
        // ...rest of the function
        Ok(())
    }
}
