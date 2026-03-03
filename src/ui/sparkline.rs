use eframe::egui;

use crate::types::format_tokens;
use crate::HistoricalData;

/// Draw a sparkline (polyline) from a series of values.
/// Returns the allocated rect for hover detection.
fn draw_sparkline(
    ui: &mut egui::Ui,
    values: &[f64],
    width: f32,
    height: f32,
    color: egui::Color32,
) -> egui::Rect {
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    if values.is_empty() {
        return rect;
    }

    let painter = ui.painter();

    // Background
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 40));

    let max_val = values.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    let n = values.len();

    if n == 1 {
        // Single point — draw a centered dot
        let center = rect.center();
        painter.circle_filled(center, 2.0, color);
        return rect;
    }

    let points: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = rect.left() + (i as f32 / (n - 1) as f32) * width;
            let y = rect.bottom() - (*v as f32 / max_val as f32) * (height - 4.0) - 2.0;
            egui::pos2(x, y)
        })
        .collect();

    // Draw the polyline
    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], egui::Stroke::new(1.5, color));
    }

    rect
}

/// Render daily sparklines row: tokens, cost, sessions.
pub fn render_daily_sparklines(ui: &mut egui::Ui, historical: &HistoricalData) {
    let totals = &historical.daily_totals;
    if totals.is_empty() {
        return;
    }

    ui.separator();
    ui.label("Historical Trends");
    ui.add_space(2.0);

    let token_values: Vec<f64> = totals.iter().map(|(_, t, _)| *t as f64).collect();
    let message_values: Vec<f64> = totals.iter().map(|(_, _, m)| *m).collect();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("Tokens/day");
            draw_sparkline(
                ui,
                &token_values,
                160.0,
                40.0,
                egui::Color32::from_rgb(100, 180, 255),
            );
            // Show total
            let total: u64 = totals.iter().map(|(_, t, _)| t).sum();
            ui.label(format!("Total: {}", format_tokens(total)));
        });

        ui.add_space(16.0);

        ui.vertical(|ui| {
            ui.label("Messages/day");
            draw_sparkline(
                ui,
                &message_values,
                160.0,
                40.0,
                egui::Color32::from_rgb(180, 130, 255),
            );
            let total: f64 = message_values.iter().sum();
            ui.label(format!("Total: {}", total as u64));
        });
    });
}

/// Draw a mini sparkline for a project trend row.
pub fn draw_project_trend(
    ui: &mut egui::Ui,
    data: &[(String, u64)],
) {
    if data.is_empty() {
        ui.label("-");
        return;
    }

    let values: Vec<f64> = data.iter().map(|(_, t)| *t as f64).collect();
    draw_sparkline(
        ui,
        &values,
        60.0,
        14.0,
        egui::Color32::from_rgb(80, 160, 255),
    );
}
