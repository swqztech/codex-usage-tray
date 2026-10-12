use crate::fonts;
use crate::usage;
use eframe::egui;
use std::time::{Duration, Instant};

pub struct UsageWindow {
    snap: usage::UsageSnapshot,
    last_refresh: Instant,
    always_on_top: bool,
}

impl UsageWindow {
    pub fn new() -> Self {
        Self {
            snap: usage::load_or_placeholder(),
            last_refresh: Instant::now(),
            always_on_top: false,
        }
    }

    fn refresh(&mut self) {
        self.snap = usage::load_or_placeholder();
        self.last_refresh = Instant::now();
    }
}

impl eframe::App for UsageWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_refresh.elapsed() >= Duration::from_secs(60) {
            self.refresh();
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.snap.taskbar_title().into()));
        let level = if self.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(24, 27, 31))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Codex 剩余用量")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(160, 165, 175)),
                    );
                    ui.add_space(6.0);

                    usage_row(
                        ui,
                        &self.snap.primary_window_label,
                        self.snap.primary_remaining,
                        self.snap.primary_resets_at,
                    );
                    usage_row(
                        ui,
                        &self.snap.secondary_window_label,
                        self.snap.secondary_remaining,
                        self.snap.secondary_resets_at,
                    );

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("刷新").clicked() {
                            self.refresh();
                        }
                        let pin_label = if self.always_on_top {
                            "取消置顶"
                        } else {
                            "置顶"
                        };
                        if ui.button(pin_label).clicked() {
                            self.always_on_top = !self.always_on_top;
                        }
                        if ui.button("退出").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    if let Some(t) = &self.snap.source_time {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(format!("更新：{}", t))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(120, 125, 135)),
                        );
                    }
                });
            });
    }
}

fn usage_row(ui: &mut egui::Ui, label: &str, remaining: f64, resets_at: Option<i64>) {
    let pct = remaining.round() as i32;
    let reset = usage::format_reset_public(resets_at);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(15.0)
                .strong()
                .color(egui::Color32::WHITE),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("重置 {}", reset))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(140, 145, 155)),
            );
            ui.label(
                egui::RichText::new(format!("{}%", pct))
                    .size(22.0)
                    .strong()
                    .color(pct_color(pct)),
            );
        });
    });
}

fn pct_color(pct: i32) -> egui::Color32 {
    if pct >= 50 {
        egui::Color32::from_rgb(45, 212, 191)
    } else if pct >= 20 {
        egui::Color32::from_rgb(250, 204, 21)
    } else {
        egui::Color32::from_rgb(248, 113, 113)
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 168.0])
            .with_min_inner_size([280.0, 150.0])
            .with_resizable(true),
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "Codex 用量",
        options,
        Box::new(|cc| {
            fonts::setup_cjk_fonts(&cc.egui_ctx);
            Ok(Box::new(UsageWindow::new()))
        }),
    )
}
