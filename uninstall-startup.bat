@echo off
set "LINK=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Codex用量监控.lnk"
if exist "%LINK%" del "%LINK%" && echo 已移除开机启动 || echo 未找到启动项
