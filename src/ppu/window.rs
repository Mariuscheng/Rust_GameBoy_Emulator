//! 視窗渲染器，產生視窗圖層像素

pub struct WindowRenderer;

impl WindowRenderer {
    pub fn render_line(&self, _line: u8) -> Vec<u8> {
        // 回傳每個像素的顏色 ID（僅範例）
        vec![0; 160]
    }
}
