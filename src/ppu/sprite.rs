//! 精靈渲染器，產生精靈圖層像素

pub struct SpriteRenderer;

impl SpriteRenderer {
    pub fn render_line(&self, _line: u8) -> Vec<Option<u8>> {
        // 回傳每個像素的顏色 ID 或 None（僅範例）
        vec![None; 160]
    }
}
