use chrono::{Local, TimeZone, Utc};
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
    pub primary_window_minutes: i64,
    pub primary_resets_at: Option<i64>,
    pub secondary_remaining: f64,
    pub secondary_window_label: String,
    pub secondary_window_minutes: i64,
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
                "{}：{}%  {}",
                self.primary_window_label,
                self.primary_remaining.round() as i32,
                format_reset_codex(self.primary_resets_at, self.primary_window_minutes),
            ),
            format!(
                "  → {} 重置",
                format_reset_clock(self.primary_resets_at),
            ),
            format!(
                "{}：{}%  {}",
                self.secondary_window_label,
                self.secondary_remaining.round() as i32,
                format_reset_codex(self.secondary_resets_at, self.secondary_window_minutes),
            ),
            format!(
                "  → {} 重置",
                format_reset_clock(self.secondary_resets_at),
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

/// 与 Codex 菜单右侧一致：5 小时等为距重置倒计时 `HH:MM`，1 周为 `M月D日`。
pub fn format_reset_codex(ts: Option<i64>, window_minutes: i64) -> String {
    match ts {
        None => "—".into(),
        Some(v) => {
            if is_short_usage_window(window_minutes) {
                format_countdown_until(normalize_unix(v))
            } else {
                format_reset_date(normalize_unix(v))
            }
        }
    }
}

/// 明确写出本地几点几分重置。
pub fn format_reset_clock(ts: Option<i64>) -> String {
    match ts {
        None => "—".into(),
        Some(v) => {
            let sec = normalize_unix(v);
            Local
                .timestamp_opt(sec, 0)
                .single()
                .map(|dt| dt.format("%m月%d日 %H:%M").to_string())
                .unwrap_or_else(|| sec.to_string())
        }
    }
}

fn is_short_usage_window(window_minutes: i64) -> bool {
    window_minutes <= 24 * 60
}

fn format_countdown_until(reset_unix: i64) -> String {
    let now = Utc::now().timestamp();
    let delta = reset_unix - now;
    if delta <= 0 {
        return "00:00".into();
    }
    let hours = delta / 3600;
    let minutes = (delta % 3600) / 60;
    format!("{:02}:{:02}", hours, minutes)
}

fn format_reset_date(reset_unix: i64) -> String {
    Local
        .timestamp_opt(reset_unix, 0)
        .single()
        .map(|dt| dt.format("%m月%d日").to_string())
        .unwrap_or_else(|| reset_unix.to_string())
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

fn normalize_unix(ts: i64) -> i64 {
    if ts > 1_000_000_000_000 {
        ts / 1000
    } else {
        ts
    }
}

/// 与 Codex 桌面端一致：窗口过了 resets_at 后配额会清零，日志里的 used_percent 可能仍是上一窗口的 stale 值。
fn window_remaining_and_next_reset(
    used: Option<f64>,
    resets_at: Option<i64>,
    window_minutes: i64,
) -> (f64, Option<i64>) {
    let now = Utc::now().timestamp();
    let window_secs = window_minutes.max(1) * 60;

    match resets_at {
        None => (remaining_from_used(used), None),
        Some(raw) => {
            let mut next_reset = normalize_unix(raw);
            if now >= next_reset {
                while next_reset <= now {
                    next_reset += window_secs;
                }
                (100.0, Some(next_reset))
            } else {
                (remaining_from_used(used), Some(next_reset))
            }
        }
    }
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
    let primary_window = primary.window_minutes.unwrap_or(300);
    let secondary_window = secondary.window_minutes.unwrap_or(10_080);
    let (primary_remaining, primary_resets_at) = window_remaining_and_next_reset(
        primary.used_percent,
        primary.resets_at,
        primary_window,
    );
    let (secondary_remaining, secondary_resets_at) = window_remaining_and_next_reset(
        secondary.used_percent,
        secondary.resets_at,
        secondary_window,
    );

    Some(UsageSnapshot {
        plan_type: limits.plan_type,
        primary_remaining,
        primary_window_label: window_label(primary_window),
        primary_window_minutes: primary_window,
        primary_resets_at,
        secondary_remaining,
        secondary_window_label: window_label(secondary_window),
        secondary_window_minutes: secondary_window,
        secondary_resets_at,
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

    let files: Vec<PathBuf> = WalkDir::new(&sessions)
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

    let mut best: Option<UsageSnapshot> = None;
    for path in &files {
        if let Some(snap) = parse_file(path) {
            best = Some(pick_newer_snapshot(best, snap));
        }
    }
    best
}

fn pick_newer_snapshot(current: Option<UsageSnapshot>, candidate: UsageSnapshot) -> UsageSnapshot {
    match current {
        None => candidate,
        Some(prev) => {
            let prev_ts = prev.source_time.as_deref().unwrap_or("");
            let cand_ts = candidate.source_time.as_deref().unwrap_or("");
            if cand_ts > prev_ts {
                candidate
            } else {
                prev
            }
        }
    }
}

pub fn load_or_placeholder() -> UsageSnapshot {
    load_latest_usage().unwrap_or_else(|| UsageSnapshot {
        plan_type: None,
        primary_remaining: 0.0,
        primary_window_label: "5 小时".into(),
        primary_window_minutes: 300,
        primary_resets_at: None,
        secondary_remaining: 0.0,
        secondary_window_label: "1 周".into(),
        secondary_window_minutes: 10_080,
        secondary_resets_at: None,
        source_file: None,
        source_time: None,
    })
}
