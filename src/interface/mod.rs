// External interface module

pub mod input;
pub mod video;
pub mod audio;

pub use input::{Joypad, GameBoyKey};
pub use video::VideoInterface;
pub use audio::AudioInterface;
