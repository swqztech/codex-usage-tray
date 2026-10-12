@echo off
chcp 65001 >nul
setlocal

set "EXE=%~dp0target\release\codex-usage-tray.exe"
if not exist "%EXE%" (
  echo 未找到 %EXE%
  echo 请先在本目录执行: cargo build --release
  exit /b 1
)

set "STARTUP=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup"
set "LINK=%STARTUP%\Codex用量监控.lnk"

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-startup.ps1"
if errorlevel 1 exit /b 1

endlocal
