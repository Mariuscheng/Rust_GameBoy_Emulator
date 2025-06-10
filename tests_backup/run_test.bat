@echo off
echo 正在執行 Game Boy 模擬器測試...
cd /d "c:\Users\mariu\Desktop\Rust\gameboy_emulator\gameboy_emulator"
cargo run -- test
if %ERRORLEVEL% EQU 0 (
    echo 測試完成！
    if exist test_result.txt (
        echo 查看測試結果：
        type test_result.txt | more
    )
) else (
    echo 測試失敗
)
pause
