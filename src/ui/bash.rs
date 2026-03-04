use eframe::egui::{self, Color32};

use crate::types::{BashCategory, MetricsState};
use super::widgets;

pub fn render(ui: &mut egui::Ui, state: &MetricsState) {
    ui.strong("Bash Analytics");
    ui.add_space(4.0);

    // Aggregate bash categories across all sessions
    let mut categories: std::collections::HashMap<BashCategory, u64> = std::collections::HashMap::new();
    let mut git_subs: std::collections::HashMap<crate::types::GitSubCommand, u64> = std::collections::HashMap::new();
    let mut total_commands: u64 = 0;
    let mut rule_violations: u64 = 0;

    for behavior in state.session_behaviors.values() {
        for (cat, count) in &behavior.bash_categories {
            *categories.entry(*cat).or_insert(0) += count;
            if *cat == BashCategory::RuleViolation {
                rule_violations += count;
            }
        }
        for (sub, count) in &behavior.git_sub_counts {
            *git_subs.entry(*sub).or_insert(0) += count;
        }
        total_commands += behavior.total_bash_commands;
    }

    if total_commands == 0 {
        ui.colored_label(Color32::from_rgb(120, 120, 120), "No bash commands yet");
        return;
    }

    widgets::render_metric_row(
        ui,
        "Total commands",
        &total_commands.to_string(),
        Color32::from_rgb(180, 180, 180),
    );
    ui.add_space(4.0);

    // Category distribution
    ui.label("Command categories:");
    ui.add_space(2.0);

    let cat_items: Vec<(BashCategory, u64, Color32, &str)> = vec![
        (BashCategory::Git, *categories.get(&BashCategory::Git).unwrap_or(&0), Color32::from_rgb(100, 180, 255), "Git"),
        (BashCategory::Test, *categories.get(&BashCategory::Test).unwrap_or(&0), Color32::from_rgb(100, 255, 100), "Test"),
        (BashCategory::Lint, *categories.get(&BashCategory::Lint).unwrap_or(&0), Color32::from_rgb(200, 200, 100), "Lint"),
        (BashCategory::PackageManager, *categories.get(&BashCategory::PackageManager).unwrap_or(&0), Color32::from_rgb(200, 150, 255), "Pkg"),
        (BashCategory::Docker, *categories.get(&BashCategory::Docker).unwrap_or(&0), Color32::from_rgb(100, 200, 200), "Docker"),
        (BashCategory::Network, *categories.get(&BashCategory::Network).unwrap_or(&0), Color32::from_rgb(255, 180, 100), "Network"),
        (BashCategory::Other, *categories.get(&BashCategory::Other).unwrap_or(&0), Color32::from_rgb(150, 150, 150), "Other"),
    ];

    let max_count = cat_items.iter().map(|(_, c, _, _)| *c).max().unwrap_or(1);

    for (_cat, count, color, label) in &cat_items {
        if *count > 0 {
            ui.horizontal(|ui| {
                ui.label(format!("{:>8}", label));
                let bar_width = if max_count > 0 {
                    (*count as f32 / max_count as f32) * 200.0
                } else {
                    0.0
                };
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(200.0, 12.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 2.0, Color32::from_rgb(40, 40, 40));
                if bar_width > 0.0 {
                    let bar = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(bar_width, rect.height()),
                    );
                    painter.rect_filled(bar, 2.0, *color);
                }
                ui.label(count.to_string());
            });
        }
    }

    // Git sub-operations
    if !git_subs.is_empty() {
        ui.add_space(8.0);
        ui.label("Git operations:");
        ui.add_space(2.0);

        let mut sorted_subs: Vec<_> = git_subs.iter().collect();
        sorted_subs.sort_by_key(|(_, c)| std::cmp::Reverse(**c));

        for (sub, count) in sorted_subs {
            let label = format!("{:?}", sub);
            widgets::render_metric_row(
                ui,
                &format!("  {}", label),
                &count.to_string(),
                Color32::from_rgb(100, 180, 255),
            );
        }
    }

    // Rule violations
    if rule_violations > 0 {
        ui.add_space(8.0);
        widgets::render_metric_row(
            ui,
            "Rule violations (cat/head/tail/sed/awk)",
            &rule_violations.to_string(),
            Color32::from_rgb(255, 80, 80),
        );
    }
}
