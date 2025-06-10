/*
================================================================================
Game Boy 模擬器 - 手柄輸入模組
================================================================================
處理手柄按鍵輸入和狀態管理

功能：
- 按鍵狀態追蹤
- 輸入事件處理
- 調試報告整合

日期: 2025年6月9日
================================================================================
*/

use chrono::Local;
use std::fs::File;
use std::io::Write;

// Game Boy 按鍵映射
#[derive(Debug, Clone, Copy)]
pub enum GameBoyKey {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

pub struct Joypad {
    // 按鍵狀態 (0 = 按下, 1 = 未按下)
    pub direction_keys: u8,     // 方向鍵狀態
    pub action_keys: u8,        // 動作鍵狀態
    pub select_direction: bool, // 是否選擇方向鍵模式
    pub select_action: bool,    // 是否選擇動作鍵模式

    // 調試相關
    debug_enabled: bool,
    debug_file: Option<File>,
    key_press_count: u64,
}

impl Joypad {
    pub fn new() -> Self {
        let debug_file = File::create("debug_report/joypad_debug.txt").ok();

        Self {
            direction_keys: 0x0F, // 所有方向鍵未按下
            action_keys: 0x0F,    // 所有動作鍵未按下
            select_direction: false,
            select_action: false,
            debug_enabled: true,
            debug_file,
            key_press_count: 0,
        }
    }

    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_enabled = enabled;
    }

    // 更新手柄狀態 (用於與主循環同步)
    pub fn update(&mut self) {
        // 這個方法在每個模擬步驟中被調用
        // 目前不需要額外的更新邏輯，但保留這個方法以便與 MMU 介面兼容
        if self.debug_enabled {
            // 每隔一段時間記錄手柄狀態
            if self.key_press_count > 0 && self.key_press_count % 1000 == 0 {
                if let Some(ref mut file) = self.debug_file {
                    let timestamp = Local::now().format("%H:%M:%S%.3f");
                    let log_entry = format!(
                        "[{}] 定期手柄狀態: 方向鍵=0x{:02X}, 動作鍵=0x{:02X}\n",
                        timestamp, self.direction_keys, self.action_keys
                    );
                    let _ = file.write_all(log_entry.as_bytes());
                    let _ = file.flush();
                }
            }
        }
    }

    // 更新按鈕狀態 (用於直接從 MMU 設置)
    pub fn update_button(&mut self, direction_keys: u8, action_keys: u8) {
        self.direction_keys = direction_keys;
        self.action_keys = action_keys;

        if self.debug_enabled {
            if let Some(ref mut file) = self.debug_file {
                let timestamp = Local::now().format("%H:%M:%S%.3f");
                let log_entry = format!(
                    "[{}] 外部更新按鈕狀態: 方向鍵=0x{:02X}, 動作鍵=0x{:02X}\n",
                    timestamp, direction_keys, action_keys
                );
                let _ = file.write_all(log_entry.as_bytes());
                let _ = file.flush();
            }
        }
    }

    // 處理按鍵按下
    pub fn key_down(&mut self, key: GameBoyKey) {
        self.key_press_count += 1;

        match key {
            GameBoyKey::Right => self.direction_keys &= !0x01,
            GameBoyKey::Left => self.direction_keys &= !0x02,
            GameBoyKey::Up => self.direction_keys &= !0x04,
            GameBoyKey::Down => self.direction_keys &= !0x08,
            GameBoyKey::A => self.action_keys &= !0x01,
            GameBoyKey::B => self.action_keys &= !0x02,
            GameBoyKey::Select => self.action_keys &= !0x04,
            GameBoyKey::Start => self.action_keys &= !0x08,
        }

        self.log_key_event(key, true);
    }

    // 處理按鍵釋放
    pub fn key_up(&mut self, key: GameBoyKey) {
        match key {
            GameBoyKey::Right => self.direction_keys |= 0x01,
            GameBoyKey::Left => self.direction_keys |= 0x02,
            GameBoyKey::Up => self.direction_keys |= 0x04,
            GameBoyKey::Down => self.direction_keys |= 0x08,
            GameBoyKey::A => self.action_keys |= 0x01,
            GameBoyKey::B => self.action_keys |= 0x02,
            GameBoyKey::Select => self.action_keys |= 0x04,
            GameBoyKey::Start => self.action_keys |= 0x08,
        }

        self.log_key_event(key, false);
    } // 讀取手柄狀態寄存器 (0xFF00)
    pub fn read_joypad_register(&mut self, select_bits: u8) -> u8 {
        self.select_direction = (select_bits & 0x10) == 0;
        self.select_action = (select_bits & 0x20) == 0;

        let mut result = 0xCF; // 高位固定為1

        if self.select_direction {
            result = (result & 0xF0) | (self.direction_keys & 0x0F);
        }

        if self.select_action {
            result = (result & 0xF0) | (self.action_keys & 0x0F);
        }

        result
    } // 寫入手柄控制寄存器
    pub fn write_joypad_register(&mut self, value: u8) {
        self.select_direction = (value & 0x10) == 0;
        self.select_action = (value & 0x20) == 0;

        if self.debug_enabled {
            self.log_register_access(value);
        }
    }

    // 獲取當前手柄狀態（用於與MMU交互）
    pub fn get_joypad_state(&self) -> u8 {
        // 返回組合的手柄狀態
        // 高4位為方向鍵，低4位為動作鍵
        (self.direction_keys << 4) | self.action_keys
    }

    // 檢查是否有按鍵按下 (用於中斷判斷)
    pub fn has_key_pressed(&self) -> bool {
        if self.select_direction && self.direction_keys != 0x0F {
            return true;
        }
        if self.select_action && self.action_keys != 0x0F {
            return true;
        }
        false
    }

    // 調試日誌：按鍵事件
    fn log_key_event(&mut self, key: GameBoyKey, pressed: bool) {
        if !self.debug_enabled {
            return;
        }

        if let Some(ref mut file) = self.debug_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let action = if pressed { "按下" } else { "釋放" };
            let key_name = match key {
                GameBoyKey::Right => "右",
                GameBoyKey::Left => "左",
                GameBoyKey::Up => "上",
                GameBoyKey::Down => "下",
                GameBoyKey::A => "A",
                GameBoyKey::B => "B",
                GameBoyKey::Select => "Select",
                GameBoyKey::Start => "Start",
            };

            let log_entry = format!(
                "[{}] 按鍵{}: {} (總按鍵次數: {})\n",
                timestamp, action, key_name, self.key_press_count
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    // 調試日誌：寄存器訪問
    fn log_register_access(&mut self, value: u8) {
        if let Some(ref mut file) = self.debug_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let log_entry = format!(
                "[{}] 手柄寄存器寫入: 0x{:02X} (方向鍵選擇: {}, 動作鍵選擇: {})\n",
                timestamp,
                value,
                if (value & 0x10) == 0 { "是" } else { "否" },
                if (value & 0x20) == 0 { "是" } else { "否" }
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    // 生成手柄狀態報告
    pub fn generate_status_report(&self) -> String {
        format!(
            "================================================================================\n\
            Game Boy 手柄狀態報告\n\
            ================================================================================\n\
            \n\
            總按鍵次數: {}\n\
            方向鍵狀態: 0x{:02X} (右:{}, 左:{}, 上:{}, 下:{})\n\
            動作鍵狀態: 0x{:02X} (A:{}, B:{}, Select:{}, Start:{})\n\
            當前選擇模式: 方向鍵={}, 動作鍵={}\n\
            \n\
            ================================================================================\n",
            self.key_press_count,
            self.direction_keys,
            if (self.direction_keys & 0x01) == 0 {
                "按下"
            } else {
                "未按"
            },
            if (self.direction_keys & 0x02) == 0 {
                "按下"
            } else {
                "未按"
            },
            if (self.direction_keys & 0x04) == 0 {
                "按下"
            } else {
                "未按"
            },
            if (self.direction_keys & 0x08) == 0 {
                "按下"
            } else {
                "未按"
            },
            self.action_keys,
            if (self.action_keys & 0x01) == 0 {
                "按下"
            } else {
                "未按"
            },
            if (self.action_keys & 0x02) == 0 {
                "按下"
            } else {
                "未按"
            },
            if (self.action_keys & 0x04) == 0 {
                "按下"
            } else {
                "未按"
            },
            if (self.action_keys & 0x08) == 0 {
                "按下"
            } else {
                "未按"
            },
            if self.select_direction {
                "啟用"
            } else {
                "停用"
            },
            if self.select_action {
                "啟用"
            } else {
                "停用"
            }
        )
    }

    // 保存最終報告到檔案
    pub fn save_final_report(&self) {
        let report_path = "debug_report/joypad_final_report.txt";
        if let Ok(mut file) = File::create(report_path) {
            let report = self.generate_status_report();
            let _ = file.write_all(report.as_bytes());
            let _ = file.flush();
            println!("手柄最終報告已生成: {}", report_path);
        }
    }

    // 重置所有按鍵狀態
    pub fn reset(&mut self) {
        self.direction_keys = 0x0F;
        self.action_keys = 0x0F;
        self.select_direction = false;
        self.select_action = false;
        self.key_press_count = 0;

        if self.debug_enabled {
            if let Some(ref mut file) = self.debug_file {
                let timestamp = Local::now().format("%H:%M:%S%.3f");
                let log_entry = format!("[{}] 手柄狀態重置\n", timestamp);
                let _ = file.write_all(log_entry.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

// 測試手柄功能的函數
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_basic_operations() {
        let mut joypad = Joypad::new();

        // 測試按鍵按下
        joypad.key_down(GameBoyKey::A);
        assert_eq!(joypad.action_keys & 0x01, 0);

        // 測試按鍵釋放
        joypad.key_up(GameBoyKey::A);
        assert_eq!(joypad.action_keys & 0x01, 1);

        // 測試寄存器讀取
        joypad.select_action = true;
        let register_value = joypad.read_joypad_register(0xDF); // 選擇動作鍵
        assert_eq!(register_value & 0x0F, joypad.action_keys);
    }
}
