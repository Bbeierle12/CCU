use eframe::egui::{self, Color32};

use crate::types::MetricsState;
use super::widgets;

pub fn render(ui: &mut egui::Ui, state: &MetricsState) {
    ui.strong("File Intelligence");
    ui.add_space(4.0);

    if state.file_intel.global_file_touches.is_empty() {
        ui.colored_label(Color32::from_rgb(120, 120, 120), "No file activity yet");
        return;
    }

    // Top files by total touches
    ui.label("Top files:");
    ui.add_space(2.0);

    let mut sorted_files: Vec<_> = state.file_intel.global_file_touches.iter().collect();
    sorted_files.sort_by_key(|(_, ft)| {
        std::cmp::Reverse(ft.read_count + ft.write_count + ft.edit_count + ft.grep_count)
    });

    egui::ScrollArea::vertical()
        .id_salt("file_heat_map")
        .max_height(200.0)
        .show(ui, |ui| {
            egui::Grid::new("file_grid")
                .num_columns(5)
                .spacing([8.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("File");
                    ui.strong("R");
                    ui.strong("W");
                    ui.strong("E");
                    ui.strong("G");
                    ui.end_row();

                    for (path, ft) in sorted_files.iter().take(20) {
                        let short = if path.len() > 40 {
                            format!("...{}", &path[path.len() - 37..])
                        } else {
                            path.to_string()
                        };
                        ui.label(short);
                        ui.colored_label(Color32::from_rgb(100, 180, 255), ft.read_count.to_string());
                        ui.colored_label(Color32::from_rgb(100, 255, 100), ft.write_count.to_string());
                        ui.colored_label(Color32::from_rgb(255, 200, 100), ft.edit_count.to_string());
                        ui.colored_label(Color32::from_rgb(200, 150, 255), ft.grep_count.to_string());
                        ui.end_row();
                    }
                });
        });

    ui.add_space(8.0);

    // File type distribution
    if !state.file_intel.extension_counts.is_empty() {
        ui.label("File types:");
        ui.add_space(2.0);

        let mut ext_sorted: Vec<_> = state.file_intel.extension_counts.iter().collect();
        ext_sorted.sort_by_key(|(_, c)| std::cmp::Reverse(**c));

        let colors = [
            Color32::from_rgb(100, 180, 255),
            Color32::from_rgb(100, 255, 100),
            Color32::from_rgb(255, 200, 100),
            Color32::from_rgb(200, 150, 255),
            Color32::from_rgb(255, 150, 150),
            Color32::from_rgb(150, 255, 200),
        ];

        let segments: Vec<(f64, Color32, &str)> = ext_sorted
            .iter()
            .take(6)
            .enumerate()
            .map(|(i, (ext, count))| (**count as f64, colors[i % colors.len()], ext.as_str()))
            .collect();

        if !segments.is_empty() {
            widgets::render_stacked_bar(ui, &segments, ui.available_width().min(400.0));
            ui.add_space(2.0);
            // Legend
            ui.horizontal_wrapped(|ui| {
                for (_, color, label) in &segments {
                    ui.colored_label(*color, format!(".{}", label));
                }
            });
        }
    }

    ui.add_space(8.0);

    // Average directory depth
    widgets::render_metric_row(
        ui,
        "Avg path depth",
        &format!("{:.1}", state.file_intel.avg_path_depth()),
        Color32::from_rgb(180, 180, 180),
    );

    // Write-then-edit ratio
    let mut total_wte: u64 = 0;
    let mut total_writes: u64 = 0;
    for behavior in state.session_behaviors.values() {
        total_wte += behavior.write_then_edit_count;
        total_writes += behavior.recently_written_files.len() as u64;
    }
    let wte_ratio = if total_writes > 0 {
        total_wte as f64 / total_writes as f64
    } else {
        0.0
    };
    widgets::render_gauge(
        ui,
        "Write-then-edit",
        wte_ratio.min(1.0),
        Color32::from_rgb(255, 200, 100),
        120.0,
    );
}
