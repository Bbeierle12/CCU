use eframe::egui;

use crate::types::MetricsState;

pub fn render(ui: &mut egui::Ui, state: &MetricsState) {
    let tools = state.tools_sorted();

    if tools.is_empty() {
        return;
    }

    ui.strong(format!("Tools ({})", tools.len()));
    ui.add_space(4.0);

    let max_count = tools.first().map(|(_, c)| **c).unwrap_or(1).max(1);

    egui::ScrollArea::vertical()
        .id_salt("tools_scroll")
        .max_height(140.0)
        .show(ui, |ui| {
            for (name, count) in &tools {
                ui.horizontal(|ui| {
                    // Tool name (16-char pad)
                    let label = if name.len() > 16 {
                        format!("{}...", &name[..13])
                    } else {
                        format!("{:<16}", name)
                    };
                    ui.label(label);

                    // Bar
                    let frac = **count as f32 / max_count as f32;
                    let bar_width = 150.0;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_width, 14.0), egui::Sense::hover());

                    let painter = ui.painter();
                    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(40, 40, 50));
                    let fill_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(bar_width * frac, 14.0),
                    );
                    painter.rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(180, 130, 255));

                    // Count label
                    ui.label(count.to_string());
                });
            }
        });
}
