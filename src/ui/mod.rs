pub mod bash;
pub mod behavior;
pub mod branches;
pub mod chains;
pub mod conversation;
pub mod cost;
pub mod files;
pub mod projects;
pub mod sessions;
pub mod settings_modal;
pub mod sparkline;
pub mod summary;
pub mod timeline;
pub mod tools;
pub mod widgets;

use eframe::egui;

use crate::settings::Settings;
use crate::types::MetricsState;
use crate::{AnalyticsTab, DateRangeSelection, HistoricalData};

/// Returns `true` if the gear (settings) button was clicked.
pub fn render(
    ctx: &egui::Context,
    state: &MetricsState,
    settings: &Settings,
    date_range: &mut DateRangeSelection,
    historical: &Option<HistoricalData>,
    session_detail: &mut sessions::SessionDetailState,
    selected_tab: &mut AnalyticsTab,
) -> bool {
    let mut gear_clicked = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Claude Code Usage");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("date_range")
                    .selected_text(date_range.label())
                    .show_ui(ui, |ui| {
                        for option in &[
                            DateRangeSelection::Today,
                            DateRangeSelection::Last7,
                            DateRangeSelection::Last30,
                            DateRangeSelection::AllTime,
                        ] {
                            ui.selectable_value(date_range, *option, option.label());
                        }
                    });

                // Gear icon for settings
                if ui.button("\u{2699}").clicked() {
                    gear_clicked = true;
                }
            });
        });
        ui.add_space(4.0);

        if state.total_messages == 0 && historical.is_none() {
            ui.centered_and_justified(|ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 120, 120),
                    "No sessions detected today",
                );
            });
            return;
        }

        // Summary + sparklines always visible
        summary::render(ui, state, settings);

        if let Some(hist) = historical {
            if !hist.daily_totals.is_empty() {
                ui.add_space(4.0);
                sparkline::render_daily_sparklines(ui, hist);
            }
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(2.0);

        // Tab bar
        ui.horizontal(|ui| {
            for tab in &[
                AnalyticsTab::Overview,
                AnalyticsTab::Behavior,
                AnalyticsTab::Files,
                AnalyticsTab::Bash,
                AnalyticsTab::Conversation,
                AnalyticsTab::Cost,
                AnalyticsTab::Chains,
            ] {
                let label = match tab {
                    AnalyticsTab::Overview => "Overview",
                    AnalyticsTab::Behavior => "Behavior",
                    AnalyticsTab::Files => "Files",
                    AnalyticsTab::Bash => "Bash",
                    AnalyticsTab::Conversation => "Convo",
                    AnalyticsTab::Cost => "Cost",
                    AnalyticsTab::Chains => "Chains",
                };
                if ui.selectable_label(*selected_tab == *tab, label).clicked() {
                    *selected_tab = *tab;
                }
            }
        });

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(4.0);

        // Tab content
        egui::ScrollArea::vertical()
            .id_salt("tab_content")
            .show(ui, |ui| {
                match selected_tab {
                    AnalyticsTab::Overview => {
                        sessions::render(ui, state, session_detail, settings);
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        projects::render(ui, state, historical);
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        tools::render(ui, state);
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        branches::render(ui, state);
                    }
                    AnalyticsTab::Behavior => behavior::render(ui, state),
                    AnalyticsTab::Files => files::render(ui, state),
                    AnalyticsTab::Bash => bash::render(ui, state),
                    AnalyticsTab::Conversation => conversation::render(ui, state),
                    AnalyticsTab::Cost => cost::render(ui, state, settings),
                    AnalyticsTab::Chains => chains::render(ui, state),
                }
            });
    });

    gear_clicked
}
