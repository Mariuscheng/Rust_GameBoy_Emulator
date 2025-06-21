/// 顏色定義
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// 像素操作 trait
pub trait Pixel {
    fn rgba(&self) -> [u8; 4];
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const LIGHT_GRAY: Self = Self::new(192, 192, 192, 255);
    pub const DARK_GRAY: Self = Self::new(96, 96, 96, 255);
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
}

impl Pixel for Color {
    fn rgba(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<u8> for Color {
    fn from(gb_color: u8) -> Self {
        match gb_color & 0x03 {
            0 => Color::WHITE,
            1 => Color::LIGHT_GRAY,
            2 => Color::DARK_GRAY,
            3 => Color::BLACK,
            _ => unreachable!(),
        }
    }
}
