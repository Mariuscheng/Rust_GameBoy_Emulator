// Joypad 按鍵定義
pub enum Button {
    Right, Left, Up, Down, A, B, Select, Start,
}

pub struct Joypad {
    pub state: u8, // 低 4 位為目前按下狀態
    pub select: u8, // 0x10: 按鈕群, 0x20: 方向群
}

impl Joypad {
    pub fn new() -> Self {
        Self { state: 0xFF, select: 0 }
    }

    pub fn set_button(&mut self, button: Button, pressed: bool) {
        let bit = match button {
            Button::Right => 0,
            Button::Left => 1,
            Button::Up => 2,
            Button::Down => 3,
            Button::A => 4,
            Button::B => 5,
            Button::Select => 6,
            Button::Start => 7,
        };
        if pressed {
            self.state &= !(1 << bit);
        } else {
            self.state |= 1 << bit;
        }
    }

    pub fn read(&self) -> u8 {
        // 依 select 回傳對應群組
        let mut res = 0xCF;
        if self.select & 0x10 == 0 {
            // 按鈕群
            res |= self.state >> 4 & 0x0F;
        }
        if self.select & 0x20 == 0 {
            // 方向群
            res |= self.state & 0x0F;
        }
        res
    }

    pub fn write(&mut self, v: u8) {
        self.select = v & 0x30;
    }
}