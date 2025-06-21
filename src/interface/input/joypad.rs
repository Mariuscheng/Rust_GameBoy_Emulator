/*
================================================================================
Game Boy Emulator - Joypad Input Module
================================================================================
Handle joypad button input and state management

Features:
- Button state tracking
- Input event handling
- Debug report integration

Date: June 9, 2025
================================================================================
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl GameBoyKey {
    pub fn is_direction(&self) -> bool {
        matches!(
            self,
            GameBoyKey::Right | GameBoyKey::Left | GameBoyKey::Up | GameBoyKey::Down
        )
    }

    pub fn is_button(&self) -> bool {
        matches!(
            self,
            GameBoyKey::A | GameBoyKey::B | GameBoyKey::Select | GameBoyKey::Start
        )
    }

    pub fn to_bit(&self) -> u8 {
        match self {
            GameBoyKey::Right => 0,
            GameBoyKey::Left => 1,
            GameBoyKey::Up => 2,
            GameBoyKey::Down => 3,
            GameBoyKey::A => 0,
            GameBoyKey::B => 1,
            GameBoyKey::Select => 2,
            GameBoyKey::Start => 3,
        }
    }
}

// Constants definition
pub const JOYPAD_DEFAULT_BUTTONS: u8 = 0xFF;
pub const JOYPAD_DEFAULT_DIRECTIONS: u8 = 0xFF;

pub trait Joypad {
    fn is_right_pressed(&self) -> bool;
    fn is_left_pressed(&self) -> bool;
    fn is_up_pressed(&self) -> bool;
    fn is_down_pressed(&self) -> bool;
    fn is_a_pressed(&self) -> bool;
    fn is_b_pressed(&self) -> bool;
    fn is_select_pressed(&self) -> bool;
    fn is_start_pressed(&self) -> bool;

    fn press_key(&mut self, key: GameBoyKey);
    fn release_key(&mut self, key: GameBoyKey);
}

pub struct JoypadImpl {
    button_states: u8,    // A, B, Select, Start
    direction_states: u8, // Right, Left, Up, Down
}

impl JoypadImpl {
    pub fn new() -> Self {
        Self {
            button_states: JOYPAD_DEFAULT_BUTTONS,
            direction_states: JOYPAD_DEFAULT_DIRECTIONS,
        }
    }

    pub fn set_button(&mut self, key: GameBoyKey, pressed: bool) {
        if key.is_button() {
            let bit = key.to_bit();
            if pressed {
                self.button_states &= !(1 << bit);
            } else {
                self.button_states |= 1 << bit;
            }
        }
    }

    pub fn set_direction(&mut self, key: GameBoyKey, pressed: bool) {
        if key.is_direction() {
            let bit = key.to_bit();
            if pressed {
                self.direction_states &= !(1 << bit);
            } else {
                self.direction_states |= 1 << bit;
            }
        }
    }
}

impl Default for JoypadImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl Joypad for JoypadImpl {
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
        if key.is_button() {
            let bit = key.to_bit();
            self.button_states &= !(1 << bit);
        } else if key.is_direction() {
            let bit = key.to_bit();
            self.direction_states &= !(1 << bit);
        }
    }

    fn release_key(&mut self, key: GameBoyKey) {
        if key.is_button() {
            let bit = key.to_bit();
            self.button_states |= 1 << bit;
        } else if key.is_direction() {
            let bit = key.to_bit();
            self.direction_states |= 1 << bit;
        }
    }
}
