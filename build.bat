@echo off
chcp 65001 > nul
cd /d "%~dp0"
echo ビルド中... (Building...)

cargo build --release

if %ERRORLEVEL% equ 0 (
    echo.
    echo ビルドが成功しました！
    copy /y "target\release\animai.exe" "animai.exe"
    echo 現在のフォルダに animai.exe をコピーしました。
    echo これと「csv」フォルダを一緒に配布すれば、他のPCでも動作します。
) else (
    echo.
    echo ビルドに失敗しました。
)
pause
