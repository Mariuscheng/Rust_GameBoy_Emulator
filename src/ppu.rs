pub struct PPU {
    pub vram: [u8; 0x2000],      // 8KB VRAM
    framebuffer: Vec<u32>,       // 160x144 畫面
}

impl PPU {
    pub fn new() -> Self {
        Self {
            vram: [0; 0x2000],
            framebuffer: vec![0xFFFFFFFF; 160 * 144],
        }
    }

    pub fn step(&mut self) {
        // 依據 VRAM 內容產生畫面（這裡簡單用 vram 前 160*144 個 byte 畫灰階）
        for y in 0..144 {
            for x in 0..160 {
                let idx = y * 160 + x;
                let v = self.vram.get(idx).copied().unwrap_or(0);
                let gray = (v as u32) * 0x010101;
                self.framebuffer[idx] = 0xFF000000 | gray;
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }
}