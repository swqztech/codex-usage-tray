#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fonts;
mod tray_mode;
mod usage;
mod window_mode;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--print" || a == "-p") {
        match usage::load_latest_usage() {
            Some(s) => {
                println!("{}", s.tray_tooltip());
                for line in s.menu_lines() {
                    println!("{}", line);
                }
            }
            None => {
                eprintln!("未找到 rate_limits，请先使用一次 Codex CLI/桌面端。");
                std::process::exit(1);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--tray") {
        tray_mode::run();
        return;
    }

    if let Err(e) = window_mode::run() {
        eprintln!("窗口启动失败: {}", e);
        std::process::exit(1);
    }
}
