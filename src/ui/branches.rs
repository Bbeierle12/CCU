use eframe::egui;

use crate::types::{format_tokens, MetricsState};

pub fn render(ui: &mut egui::Ui, state: &MetricsState) {
    let branches = state.branches_sorted();

    if branches.is_empty() {
        return;
    }

    ui.strong(format!("Branches ({})", branches.len()));
    ui.add_space(4.0);

    let max_tokens = branches
        .first()
        .map(|b| b.total_tokens())
        .unwrap_or(1)
        .max(1);

    egui::ScrollArea::vertical()
        .id_salt("branches_scroll")
        .max_height(120.0)
        .show(ui, |ui| {
            for b in &branches {
                ui.horizontal(|ui| {
                    // Branch name (24-char, truncate+ellipsis)
                    let label = if b.name.len() > 24 {
                        format!("{}...", &b.name[..21])
                    } else {
                        format!("{:<24}", b.name)
                    };
                    ui.label(label);

                    // Bar
                    let frac = b.total_tokens() as f32 / max_tokens as f32;
                    let bar_width = 120.0;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_width, 14.0), egui::Sense::hover());

                    let painter = ui.painter();
                    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(40, 40, 50));
                    let fill_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(bar_width * frac, 14.0),
                    );
                    painter.rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(100, 200, 140));

                    // Token count + message count
                    ui.label(format!(
                        "{} ({} msgs)",
                        format_tokens(b.total_tokens()),
                        b.message_count
                    ));
                });
            }
        });
}
