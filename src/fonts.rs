use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::path::PathBuf;
use std::sync::Arc;

/// 加载 Windows 常见中文字体，避免界面显示为方框。
pub fn setup_cjk_fonts(ctx: &egui::Context) {
    let bytes = load_system_cjk_font();
    if bytes.is_empty() {
        return;
    }

    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), Arc::new(FontData::from_owned(bytes)));

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "cjk".to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "cjk".to_owned());

    ctx.set_fonts(fonts);
}

fn load_system_cjk_font() -> Vec<u8> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
    let fonts_dir = PathBuf::from(windir).join("Fonts");
    let candidates = [
        "msyh.ttc",   // 微软雅黑
        "msyhbd.ttc",
        "simhei.ttf", // 黑体
        "simsun.ttc", // 宋体
    ];
    for name in candidates {
        let path = fonts_dir.join(name);
        if let Ok(data) = std::fs::read(&path) {
            return data;
        }
    }
    Vec::new()
}
