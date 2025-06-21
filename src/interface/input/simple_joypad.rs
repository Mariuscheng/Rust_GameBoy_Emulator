/*
================================================================================
Game Boy Emulator - Simple Joypad Input Module
================================================================================
Provides simple joypad control implementation, no direct MMU and interrupt register access needed

Date: June 18, 2025
================================================================================
*/

use crate::interface::input::joypad::{
    GameBoyKey, Joypad, JOYPAD_DEFAULT_BUTTONS, JOYPAD_DEFAULT_DIRECTIONS,
};

#[derive(Debug)]
pub struct SimpleJoypad {
    button_states: u8,
    direction_states: u8,
}

impl SimpleJoypad {
    /// Create a simple version of Joypad without using MMU and interrupt registers
    pub fn new() -> Self {
        Self {
            button_states: JOYPAD_DEFAULT_BUTTONS,
            direction_states: JOYPAD_DEFAULT_DIRECTIONS,
        }
    }

    /// Get current key state as byte representation
    pub fn get_state(&self) -> u8 {
        // Return combined state
        (self.button_states & 0x0F) | ((self.direction_states & 0x0F) << 4)
    }
}

impl Joypad for SimpleJoypad {
    fn is_right_pressed(&self) -> bool {
        (self.direction_states & (1 << GameBoyKey::Right.to_bit())) == 0
    }

    fn is_left_pressed(&self) -> bool {
        (self.direction_states & (1 << GameBoyKey::Left.to_bit())) == 0
    }

    fn is_up_pressed(&self) -> bool {
        (self.direction_states & (1 << GameBoyKey::Up.to_bit())) == 0
    }

    fn is_down_pressed(&self) -> bool {
        (self.direction_states & (1 << GameBoyKey::Down.to_bit())) == 0
    }

    fn is_a_pressed(&self) -> bool {
        (self.button_states & (1 << GameBoyKey::A.to_bit())) == 0
    }

    fn is_b_pressed(&self) -> bool {
        (self.button_states & (1 << GameBoyKey::B.to_bit())) == 0
    }

    fn is_select_pressed(&self) -> bool {
        (self.button_states & (1 << GameBoyKey::Select.to_bit())) == 0
    }

    fn is_start_pressed(&self) -> bool {
        (self.button_states & (1 << GameBoyKey::Start.to_bit())) == 0
    }

    fn press_key(&mut self, key: GameBoyKey) {
        let bit = key.to_bit();
        if key.is_button() {
            self.button_states &= !(1 << bit);
        } else {
            self.direction_states &= !(1 << bit);
        }
    }

    fn release_key(&mut self, key: GameBoyKey) {
        let bit = key.to_bit();
        if key.is_button() {
            self.button_states |= 1 << bit;
        } else {
            self.direction_states |= 1 << bit;
        }
    }
}
