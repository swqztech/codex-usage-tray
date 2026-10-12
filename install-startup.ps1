$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $root "target\release\codex-usage-tray.exe"
if (-not (Test-Path $exe)) {
    Write-Host "未找到 $exe ，请先执行: cargo build --release"
    exit 1
}

$startup = [Environment]::GetFolderPath("Startup")
$linkPath = Join-Path $startup "Codex用量监控.lnk"

$ws = New-Object -ComObject WScript.Shell
$shortcut = $ws.CreateShortcut($linkPath)
$shortcut.TargetPath = $exe
$shortcut.WorkingDirectory = Split-Path $exe -Parent
$shortcut.WindowStyle = 7
$shortcut.Description = "Codex 剩余用量小窗"
$shortcut.Save()

Write-Host "已添加到开机启动: $linkPath"
