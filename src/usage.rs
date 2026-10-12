use chrono::{Local, TimeZone};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct UsageSnapshot {
    pub plan_type: Option<String>,
    pub primary_remaining: f64,
    pub primary_window_label: String,
    pub primary_resets_at: Option<i64>,
    pub secondary_remaining: f64,
    pub secondary_window_label: String,
    pub secondary_resets_at: Option<i64>,
    #[allow(dead_code)]
    pub source_file: Option<PathBuf>,
    pub source_time: Option<String>,
}

impl UsageSnapshot {
    pub fn tray_tooltip(&self) -> String {
        format!(
            "Codex 剩余 · {} {}% | {} {}%",
            self.primary_window_label,
            self.primary_remaining.round() as i32,
            self.secondary_window_label,
            self.secondary_remaining.round() as i32,
        )
    }

    /// 任务栏按钮上显示的短标题
    pub fn taskbar_title(&self) -> String {
        format!(
            "Codex {}% · {} {}",
            self.primary_remaining.round() as i32,
            self.secondary_window_label,
            self.secondary_remaining.round() as i32,
        )
    }

    pub fn menu_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "{}：剩余 {}%（{} 重置）",
                self.primary_window_label,
                self.primary_remaining.round() as i32,
                format_reset(self.primary_resets_at),
            ),
            format!(
                "{}：剩余 {}%（{} 重置）",
                self.secondary_window_label,
                self.secondary_remaining.round() as i32,
                format_reset(self.secondary_resets_at),
            ),
        ];
        if let Some(plan) = &self.plan_type {
            lines.push(format!("套餐：{}", plan));
        }
        if let Some(t) = &self.source_time {
            lines.push(format!("数据时间：{}", t));
        }
        lines
    }
}

pub fn format_reset_public(ts: Option<i64>) -> String {
    format_reset(ts)
}

fn format_reset(ts: Option<i64>) -> String {
    match ts {
        None => "—".into(),
        Some(v) if v > 1_000_000_000_000 => {
            let sec = v / 1000;
            Local
                .timestamp_opt(sec, 0)
                .single()
                .map(|dt| dt.format("%m月%d日 %H:%M").to_string())
                .unwrap_or_else(|| v.to_string())
        }
        Some(v) => Local
            .timestamp_opt(v, 0)
            .single()
            .map(|dt| dt.format("%m月%d日 %H:%M").to_string())
            .unwrap_or_else(|| v.to_string()),
    }
}

fn window_label(minutes: i64) -> String {
    if minutes == 300 {
        "5 小时".into()
    } else if minutes == 10_080 {
        "1 周".into()
    } else if minutes % 1440 == 0 {
        format!("{} 天", minutes / 1440)
    } else if minutes % 60 == 0 {
        format!("{} 小时", minutes / 60)
    } else {
        format!("{} 分钟", minutes)
    }
}

#[derive(Deserialize)]
struct WindowLimit {
    used_percent: Option<f64>,
    window_minutes: Option<i64>,
    resets_at: Option<i64>,
}

#[derive(Deserialize)]
struct RateLimits {
    plan_type: Option<String>,
    primary: Option<WindowLimit>,
    secondary: Option<WindowLimit>,
}

fn remaining_from_used(used: Option<f64>) -> f64 {
    let u = used.unwrap_or(0.0).clamp(0.0, 100.0);
    (100.0 - u).clamp(0.0, 100.0)
}

fn parse_line(line: &str, path: &Path) -> Option<UsageSnapshot> {
    let root: Value = serde_json::from_str(line).ok()?;
    let rate_val = root
        .pointer("/payload/rate_limits")
        .or_else(|| root.get("rate_limits"))?;
    let limits: RateLimits = serde_json::from_value(rate_val.clone()).ok()?;
    let primary = limits.primary?;
    let secondary = limits.secondary?;
    let ts = root
        .get("timestamp")
        .and_then(|v| v.as_str())
        .map(String::from);
    Some(UsageSnapshot {
        plan_type: limits.plan_type,
        primary_remaining: remaining_from_used(primary.used_percent),
        primary_window_label: window_label(primary.window_minutes.unwrap_or(300)),
        primary_resets_at: primary.resets_at,
        secondary_remaining: remaining_from_used(secondary.used_percent),
        secondary_window_label: window_label(secondary.window_minutes.unwrap_or(10_080)),
        secondary_resets_at: secondary.resets_at,
        source_file: Some(path.to_path_buf()),
        source_time: ts,
    })
}

fn parse_file(path: &Path) -> Option<UsageSnapshot> {
    let text = fs::read_to_string(path).ok()?;
    let mut latest: Option<UsageSnapshot> = None;
    for line in text.lines() {
        if !line.contains("rate_limits") {
            continue;
        }
        if let Some(snap) = parse_line(line, path) {
            latest = Some(snap);
        }
    }
    latest
}

pub fn codex_home() -> PathBuf {
    if let Ok(p) = std::env::var("CODEX_HOME") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    dirs::home_dir()
        .map(|h| h.join(".codex"))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

/// 从会话 rollout 日志中取最近一次 rate_limits（桌面端与 CLI 共用）
pub fn load_latest_usage() -> Option<UsageSnapshot> {
    let sessions = codex_home().join("sessions");
    if !sessions.is_dir() {
        return None;
    }

    let mut files: Vec<PathBuf> = WalkDir::new(&sessions)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    files.sort_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    files.reverse();

    for path in files.iter().take(80) {
        if let Some(snap) = parse_file(path) {
            return Some(snap);
        }
    }
    None
}

pub fn load_or_placeholder() -> UsageSnapshot {
    load_latest_usage().unwrap_or_else(|| UsageSnapshot {
        plan_type: None,
        primary_remaining: 0.0,
        primary_window_label: "5 小时".into(),
        primary_resets_at: None,
        secondary_remaining: 0.0,
        secondary_window_label: "1 周".into(),
        secondary_resets_at: None,
        source_file: None,
        source_time: None,
    })
}
