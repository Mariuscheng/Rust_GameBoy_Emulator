use std::cell::RefCell;
use std::rc::Rc;

/// VRAM（視訊記憶體）型別定義
#[derive(Debug, Clone)]
pub struct VRAM {
    data: Vec<u8>,
}

impl VRAM {
    pub fn new() -> Self {
        Self {
            data: vec![0; 0x2000],  // 8KB VRAM
        }
    }

    /// 讀取指定位置的資料
    pub fn read(&self, addr: u16) -> u8 {
        self.data[(addr - 0x8000) as usize]
    }

    /// 寫入資料到指定位置
    pub fn write(&mut self, addr: u16, value: u8) {
        let index = (addr - 0x8000) as usize;
        if index < self.data.len() {
            self.data[index] = value;
        }
    }

    /// 取得單個圖塊（Tile）資料
    pub fn get_tile_data(&self, tile_id: u8, start_addr: u16) -> [u8; 16] {
        let mut tile_data = [0u8; 16];
        let tile_offset = (tile_id as u16) * 16;
        let start_address = start_addr.wrapping_add(tile_offset);

        for i in 0..16 {
            let addr = (start_address.wrapping_add(i) - 0x8000) as usize;
            if addr < self.data.len() {
                tile_data[i as usize] = self.data[addr];
            }
        }

        tile_data
    }

    /// 取得背景圖塊地圖
    pub fn get_background_map(&self, is_high: bool) -> &[u8] {
        let start = if is_high { 0x1C00 } else { 0x1800 };
        &self.data[start..start + 0x400]
    }

    /// 取得視窗圖塊地圖
    pub fn get_window_map(&self, is_high: bool) -> &[u8] {
        let start = if is_high { 0x1C00 } else { 0x1800 };
        &self.data[start..start + 0x400]
    }
}
