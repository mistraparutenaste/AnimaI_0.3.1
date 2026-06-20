@echo off
chcp 65001 > nul
cd /d "%~dp0"
echo 起動中... (Starting AnimaI T2I...)

REM 実行ファイルがすでにある場合（配布用・ビルド済みの場合）
if exist "animai.exe" (
    start "" "animai.exe"
    exit
)

REM Cargo (Rust) がインストールされている開発環境の場合
cargo run --release
if %ERRORLEVEL% neq 0 (
    echo.
    echo 起動に失敗しました。Rust環境が正しくインストールされているか確認してください。
    pause
)
