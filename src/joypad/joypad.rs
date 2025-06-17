/*
================================================================================
Game Boy 模擬器 - 手柄輸入模組
================================================================================
處理手柄按鍵輸入和狀態管理

功能：
- 按鍵狀態追蹤
- 輸入事件處理
- 狀態紀錄

日期: 2025年6月15日
================================================================================
*/

use crate::cpu::interrupts::InterruptRegisters;
use crate::mmu::MMU;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub enum GameBoyKey {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
    A = 4,
    B = 5,
    Select = 6,
    Start = 7,
}

pub struct Joypad {
    mmu: Rc<RefCell<MMU>>,
    direction_keys: u8,
    action_keys: u8,
    last_write: u8,
    interrupt_registers: Rc<RefCell<InterruptRegisters>>,
}

impl Joypad {
    pub fn new(interrupt_registers: Rc<RefCell<InterruptRegisters>>) -> Self {
        Joypad {
            mmu: Rc::new(RefCell::new(MMU::new(Vec::new()))),
            direction_keys: 0xFF,
            action_keys: 0xFF,
            last_write: 0xFF,
            interrupt_registers,
        }
    }

    pub fn set_mmu(&mut self, mmu: Rc<RefCell<MMU>>) {
        self.mmu = mmu;
    }

    pub fn set_key_state(&mut self, key: GameBoyKey, pressed: bool) {
        let bit = 1 << (key as u8 & 0x7);
        let was_high = match key {
            GameBoyKey::Right | GameBoyKey::Left | GameBoyKey::Up | GameBoyKey::Down => {
                (self.direction_keys & bit) != 0
            }
            _ => (self.action_keys & bit) != 0,
        };

        if pressed {
            match key {
                GameBoyKey::Right | GameBoyKey::Left | GameBoyKey::Up | GameBoyKey::Down => {
                    self.direction_keys &= !bit;
                }
                _ => {
                    self.action_keys &= !bit;
                }
            }
        } else {
            match key {
                GameBoyKey::Right | GameBoyKey::Left | GameBoyKey::Up | GameBoyKey::Down => {
                    self.direction_keys |= bit;
                }
                _ => {
                    self.action_keys |= bit;
                }
            }
        }

        // 檢查是否需要觸發中斷
        if was_high && pressed {
            self.interrupt_registers.borrow_mut().request_interrupt(4); // Joypad 中斷
        }
    }

    pub fn read(&self) -> u8 {
        let mut value = self.last_write | 0xCF;

        if (value & 0x10) == 0 {
            // 方向鍵選擇位
            value &= self.direction_keys;
        }
        if (value & 0x20) == 0 {
            // 動作鍵選擇位
            value &= self.action_keys;
        }

        value
    }

    pub fn write(&mut self, value: u8) {
        self.last_write = value;
    }

    pub fn reset(&mut self) {
        self.direction_keys = 0xFF;
        self.action_keys = 0xFF;
        self.last_write = 0xFF;
    }

    pub fn step(&mut self) -> Result<(), String> {
        // TODO: 更新按鍵狀態
        Ok(())
    }

    pub fn press(&mut self, button: GameBoyKey) {
        match button {
            GameBoyKey::Right | GameBoyKey::Left | GameBoyKey::Up | GameBoyKey::Down => {
                self.direction_keys &= !(1 << (button as u8));
            }
            GameBoyKey::A | GameBoyKey::B | GameBoyKey::Select | GameBoyKey::Start => {
                self.action_keys &= !(1 << (button as u8));
            }
        }
        self.update_joypad_register();
    }

    pub fn release(&mut self, button: GameBoyKey) {
        match button {
            GameBoyKey::Right | GameBoyKey::Left | GameBoyKey::Up | GameBoyKey::Down => {
                self.direction_keys |= 1 << (button as u8);
            }
            GameBoyKey::A | GameBoyKey::B | GameBoyKey::Select | GameBoyKey::Start => {
                self.action_keys |= 1 << (button as u8);
            }
        }
        self.update_joypad_register();
    }

    pub fn set_button_state(&mut self, button: GameBoyKey, pressed: bool) {
        let bit = 1 << (button as u8);
        match button {
            GameBoyKey::Up | GameBoyKey::Down | GameBoyKey::Left | GameBoyKey::Right => {
                if pressed {
                    self.direction_keys &= !bit;
                } else {
                    self.direction_keys |= bit;
                }
            }
            GameBoyKey::A | GameBoyKey::B | GameBoyKey::Select | GameBoyKey::Start => {
                if pressed {
                    self.action_keys &= !bit;
                } else {
                    self.action_keys |= bit;
                }
            }
        }
        self.update_joypad_register();
    }

    pub fn set_button_right(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::Right, pressed);
    }

    pub fn set_button_left(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::Left, pressed);
    }

    pub fn set_button_up(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::Up, pressed);
    }

    pub fn set_button_down(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::Down, pressed);
    }

    pub fn set_button_a(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::A, pressed);
    }

    pub fn set_button_b(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::B, pressed);
    }

    pub fn set_button_select(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::Select, pressed);
    }

    pub fn set_button_start(&mut self, pressed: bool) {
        self.set_button_state(GameBoyKey::Start, pressed);
    }

    fn update_joypad_register(&mut self) {
        let mut value = self.last_write;
        if value & 0x10 == 0 {
            value &= self.direction_keys;
        }
        if value & 0x20 == 0 {
            value &= self.action_keys;
        }
        let _ = self.mmu.borrow_mut().write_byte(0xFF00, value);
    }
}

// 測試手柄功能的函數
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_basic_operations() {
        let interrupt_registers = Rc::new(RefCell::new(InterruptRegisters::new()));
        let mut joypad = Joypad::new(interrupt_registers);

        // 測試按鍵按下
        joypad.press(GameBoyKey::A);
        assert_eq!(joypad.action_keys & 0x01, 0);

        // 測試按鍵釋放
        joypad.release(GameBoyKey::A);
        assert_eq!(joypad.action_keys & 0x01, 1);

        // 測試寄存器讀取
        joypad.last_write = 0xDF; // 模擬寫入選擇動作鍵
        let register_value = joypad.read(); // 讀取寄存器
        assert_eq!(register_value & 0x0F, joypad.action_keys);
    }
}

impl Clone for Joypad {
    fn clone(&self) -> Self {
        Self {
            mmu: Rc::clone(&self.mmu),
            direction_keys: self.direction_keys,
            action_keys: self.action_keys,
            last_write: self.last_write,
            interrupt_registers: Rc::clone(&self.interrupt_registers),
        }
    }
}
