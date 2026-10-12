# codex-usage-tray

默认以**小窗口**显示 **Codex 剩余用量**（会出现在 Windows **任务栏**上，可拖到任意显示器）；也可用 `--tray` 仅托盘。数据来自本机 `~/.codex/sessions/**/rollout-*.jsonl` 里最近的 `rate_limits`。

## 编译

需安装 [Rust](https://rustup.rs/)：

```bash
cd codex-usage-tray
cargo build --release
```

产物：`target/release/codex-usage-tray.exe`

## 开机自启（Windows）

先编译 release，再执行：

```bat
install-startup.bat
```

会在「启动」文件夹创建快捷方式。取消：`uninstall-startup.bat`。

## 运行

双击或在终端：

```bash
target\release\codex-usage-tray.exe
```

- 窗口大字显示 5 小时 / 1 周剩余与重置时间；任务栏标题同步百分比
- 拖到副屏后，按钮一般出现在**该显示器任务栏**（需系统开启「在所有显示器上显示任务栏」）
- 支持置顶、刷新、退出；窗口位置会记住（`persist_window`）
- 每 60 秒自动刷新

## 环境变量

- `CODEX_HOME`：默认 `%USERPROFILE%\.codex`

## 说明

- 若长时间未使用 Codex，日志里可能没有新 `rate_limits`，显示会偏旧；在桌面端或 CLI 用一次 Codex 后会更新。
- 本工具只读本地日志，不上传账号信息。
