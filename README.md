# Game Boy Emulator(未完成)

這是一個用 Rust 語言編寫的 Game Boy 模擬器專案。該模擬器旨在模擬原始 Game Boy 的硬體行為，並能夠運行 Game Boy 遊戲。目前已經能夠運行多種Game Boy遊戲，如俄羅斯方塊(Tetris)和超級瑪莉歐大陸(Super Mario Land)。

## 專案結構

- `src/main.rs`: 應用程式的入口點，初始化和協調各硬體元件，並管理主循環
- `src/cpu.rs`: CPU模擬實現，包含完整的Game Boy指令集和中斷處理
- `src/mmu.rs`: 記憶體管理單元，處理整個系統的記憶體映射和訪問
- `src/ppu.rs`: 像素處理單元，負責圖形渲染和顯示
- `src/apu.rs`: 音訊處理單元，負責聲音生成（目前功能有限）
- `src/joypad.rs`: 控制器輸入處理
- `src/timer.rs`: 系統計時器實現
- `src/memory_viewer.rs`: 記憶體調試工具
- `src/mmu/mbc.rs`: 記憶體庫控制器(MBC)實現，支援不同的卡帶類型
- `Cargo.toml`: 專案配置文件

## 安裝與運行

1. 確保已安裝 Rust 環境。可以通過 [rustup](https://rustup.rs/) 進行安裝。
2. 下載或克隆此專案。
3. 在專案根目錄下運行以下命令以編譯專案：

   ```
   cargo build
   ```

4. 編譯完成後，使用ROM文件運行模擬器：

   ```
   cargo run -- path/to/your/rom.gb
   ```

   例如：

   ```
   cargo run -- rom/tetris.gb
   ```

## 使用說明

### 控制鍵
- **方向鍵**: 控制遊戲中的方向
- **Z鍵**: Game Boy的A鈕
- **X鍵**: Game Boy的B鈕
- **Enter鍵**: Game Boy的Start鈕
- **Space鍵**: Game Boy的Select鈕
- **Esc鍵**: 退出模擬器

### 支援的遊戲
目前已經測試並確認可以運行的遊戲:
- Tetris
- Super Mario Land

### 已實現的功能
- CPU: 完整的Game Boy指令集，包括標準指令和CB前綴指令
- MMU: 完整的記憶體管理，支援不同卡帶類型(MBC1等)
- PPU: 基本的圖形渲染，包括背景、視窗和精靈
- 輸入: 完整的控制器輸入處理

### 待實現的功能
- 聲音支援: APU模組尚未完全實現
- 保存狀態功能
- 更多的MBC支援

## 特別感謝
- [Game Boy CPU Manual](http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf)
- [Pan Docs](https://gbdev.io/pandocs/)
- [Game Boy 開發者社群](https://gbdev.io/)

## 貢獻

歡迎任何形式的貢獻！如果您有任何建議或發現問題，請隨時提出。
