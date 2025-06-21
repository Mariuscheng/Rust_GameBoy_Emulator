//! 視窗渲染器，產生視窗圖層像素

#[derive(Debug, Default)]
pub struct WindowRenderer {
    enabled: bool,
    x: u8,
    y: u8,
}

impl WindowRenderer {
    pub fn new() -> Self {
        Self {
            enabled: false,
            x: 0,
            y: 0,
        }
    }

    pub fn render_line(&self, _line: u8) -> Vec<u8> {
        // 回傳每個像素的顏色 ID（僅範例）
        vec![0; 160]
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_position(&mut self, x: u8, y: u8) {
        self.x = x;
        self.y = y;
    }
}
