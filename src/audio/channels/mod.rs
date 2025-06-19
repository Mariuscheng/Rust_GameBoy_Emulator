//! Game Boy 音效通道模組
//! 提供四種基本音效通道的實現

// 內部實現
mod implementations;

// Re-exports
pub use implementations::{
    Noise,   // 噪音通道
    Square1, // 方波通道 1 (帶掃頻)
    Square2, // 方波通道 2
    Wave,    // 波表通道
};
