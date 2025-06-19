/*
================================================================================
Game Boy 模擬器 - 簡易版手柄輸入模組
================================================================================
提供簡易版的手柄控制實現，無需直接提供 MMU 和中斷寄存器

日期: 2025年6月18日
================================================================================
*/

use crate::joypad::GameBoyKey;
use crate::joypad::Joypad;

// 為 Joypad 添加一個簡易版的建構函數
impl Joypad {
    /// 創建一個簡易版本的 Joypad，不使用 MMU 和中斷寄存器
    pub fn new_simple() -> Self {
        // 直接使用基本的建構函數建立
        Self::new()
    }

    /// 模擬按下特定按鍵
    pub fn press_key(&mut self, key: GameBoyKey) {
        self.press(key);
    }

    /// 模擬釋放特定按鍵
    pub fn release_key(&mut self, key: GameBoyKey) {
        self.release(key);
    }
}
