use eframe::egui;

use crate::types::{format_tokens, MetricsState};
use crate::HistoricalData;

use super::sparkline;

pub fn render(ui: &mut egui::Ui, state: &MetricsState, historical: &Option<HistoricalData>) {
    let projects = state.projects_sorted();

    if projects.is_empty() {
        return;
    }

    ui.strong(format!("Projects ({})", projects.len()));
    ui.add_space(4.0);

    let max_tokens = projects.first().map(|p| p.total_tokens()).unwrap_or(1).max(1);
    let has_trends = historical
        .as_ref()
        .is_some_and(|h| !h.project_trends.is_empty());

    egui::ScrollArea::vertical()
        .id_salt("projects_scroll")
        .max_height(160.0)
        .show(ui, |ui| {
            for p in &projects {
                ui.horizontal(|ui| {
                    // Project name
                    let name = if p.name.len() > 24 {
                        format!("{}...", &p.name[..21])
                    } else {
                        p.name.clone()
                    };
                    ui.label(format!("{:<24}", name));

                    // Bar
                    let frac = p.total_tokens() as f32 / max_tokens as f32;
                    let bar_width = if has_trends { 100.0 } else { 150.0 };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_width, 14.0), egui::Sense::hover());

                    let painter = ui.painter();
                    // Background
                    painter.rect_filled(
                        rect,
                        2.0,
                        egui::Color32::from_rgb(40, 40, 50),
                    );
                    // Fill
                    let fill_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(bar_width * frac, 14.0),
                    );
                    painter.rect_filled(
                        fill_rect,
                        2.0,
                        egui::Color32::from_rgb(80, 160, 255),
                    );

                    // Token count label
                    ui.label(format_tokens(p.total_tokens()));

                    // Trend sparkline (if historical data available)
                    if let Some(hist) = historical {
                        if let Some(trend) = hist.project_trends.get(&p.name) {
                            sparkline::draw_project_trend(ui, trend);
                        }
                    }
                });
            }
        });
}
