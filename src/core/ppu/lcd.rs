//! LCD 控制器模組，管理 LCD 狀態與暫存器

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LCDMode {
    HBlank = 0,
    VBlank = 1,
    OAMScan = 2,
    Drawing = 3,
}

#[derive(Debug, Clone, Copy)]
pub struct LCDControl(u8);

impl LCDControl {
    pub fn new(value: u8) -> Self {
        LCDControl(value)
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn display_enable(&self) -> bool {
        (self.0 & 0x80) != 0
    }

    pub fn window_tilemap(&self) -> bool {
        (self.0 & 0x40) != 0
    }

    pub fn window_enable(&self) -> bool {
        (self.0 & 0x20) != 0
    }

    pub fn bg_window_tiledata(&self) -> bool {
        (self.0 & 0x10) != 0
    }

    pub fn bg_tilemap(&self) -> bool {
        (self.0 & 0x08) != 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LCDStatus(u8);

impl LCDStatus {
    pub fn new(value: u8) -> Self {
        LCDStatus(value)
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn get_mode(&self) -> LCDMode {
        match self.0 & 0x03 {
            0 => LCDMode::HBlank,
            1 => LCDMode::VBlank,
            2 => LCDMode::OAMScan,
            3 => LCDMode::Drawing,
            _ => unreachable!(),
        }
    }

    pub fn set_mode(&mut self, mode: LCDMode) {
        self.0 = (self.0 & 0xFC) | (mode as u8);
    }
}

pub struct LCD {
    control: LCDControl,
    status: LCDStatus,
}

impl LCD {
    pub fn new() -> Self {
        Self {
            control: LCDControl::new(0x91),  // Default value
            status: LCDStatus::new(0x00),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.control.display_enable()
    }

    pub fn get_mode(&self) -> LCDMode {
        self.status.get_mode()
    }

    pub fn set_mode(&mut self, mode: LCDMode) {
        self.status.set_mode(mode);
    }
}
