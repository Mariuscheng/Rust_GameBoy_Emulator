
    /// 手動寫入測試模式到 VRAM（根據 Fix_blank_screen.md 建議）
    pub fn write_test_pattern_to_vram(&mut self) {
        println!(" 手動寫入測試模式到 VRAM...");
        
        // Write a simple test pattern to first tile
        let mut vram = self.vram.borrow_mut();
        
        // First tile: solid black (all 1s)
        for i in 0..16 {
            vram[i] = 0xFF;
        }
        
        // Second tile: checkerboard
        for i in (16..32).step_by(2) {
            vram[i] = 0xAA;
            vram[i+1] = 0x55;
        }
        
        // Third tile: horizontal stripes
        for i in (32..48).step_by(4) {
            vram[i] = 0xFF;
            vram[i+1] = 0xFF;
            vram[i+2] = 0x00;
            vram[i+3] = 0x00;
        }
        
        // Make first few tiles in BG map point to these test tiles
        for i in 0..10 {
            vram[0x1800 + i] = (i % 3) as u8; // 使用前3個測試瓦片
        }
        
        println!(" 測試模式寫入完成:");
        println!("  - Tile 0: 實心黑色");
        println!("  - Tile 1: 棋盤模式");
        println!("  - Tile 2: 水平條紋");
        println!("  - 背景地圖設定為循環使用這些瓦片");
    }

