use chrono::Utc;
use eframe::egui;

use crate::settings::Settings;
use crate::types::{format_tokens, MetricsState};

use super::timeline;

/// Expanded session detail state. Stored externally and passed in.
pub struct SessionDetailState {
    pub expanded_session: Option<String>,
    pub timeline_events: Vec<timeline::TimelineEvent>,
    pub projects_dir: std::path::PathBuf,
}

impl SessionDetailState {
    pub fn new(projects_dir: std::path::PathBuf) -> Self {
        Self {
            expanded_session: None,
            timeline_events: Vec::new(),
            projects_dir,
        }
    }

    pub fn toggle(&mut self, session_id: &str) {
        if self.expanded_session.as_deref() == Some(session_id) {
            self.expanded_session = None;
            self.timeline_events.clear();
        } else {
            self.expanded_session = Some(session_id.to_string());
            self.timeline_events =
                timeline::load_session_timeline(&self.projects_dir, session_id);
        }
    }
}

pub fn render(ui: &mut egui::Ui, state: &MetricsState, detail: &mut SessionDetailState, settings: &Settings) {
    let sessions = state.sessions_sorted();
    let now = Utc::now();

    // We also need session IDs, so collect from the main state
    let session_ids: Vec<String> = {
        let mut sorted: Vec<_> = state.sessions.iter().collect();
        sorted.sort_by_key(|(_, s)| std::cmp::Reverse(s.last_seen));
        sorted.into_iter().map(|(id, _)| id.clone()).collect()
    };

    if sessions.is_empty() {
        ui.colored_label(
            egui::Color32::from_rgb(120, 120, 120),
            "No sessions today",
        );
        return;
    }

    ui.strong(format!("Sessions ({})", sessions.len()));
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("sessions_scroll")
        .max_height(200.0)
        .show(ui, |ui| {
            egui::Grid::new("sessions_grid")
                .num_columns(6)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Project");
                    ui.strong("Model");
                    ui.strong("Duration");
                    ui.strong("Tokens");
                    ui.strong("Cost");
                    ui.strong("Branch");
                    ui.end_row();

                    for (i, s) in sessions.iter().enumerate() {
                        let session_id = session_ids.get(i).map(|s| s.as_str()).unwrap_or("");
                        let is_active =
                            s.is_active(now, settings.active_session_threshold_minutes);
                        let is_expanded = detail.expanded_session.as_deref() == Some(session_id);
                        let text_color = if is_active {
                            egui::Color32::from_rgb(200, 230, 255)
                        } else {
                            egui::Color32::from_rgb(180, 180, 180)
                        };

                        // Project name (clickable)
                        let proj = if s.project.len() > 20 {
                            format!("{}...", &s.project[..17])
                        } else {
                            s.project.clone()
                        };
                        let prefix = if is_expanded { "v " } else { "> " };
                        if ui
                            .add(egui::Label::new(
                                egui::RichText::new(format!("{}{}", prefix, proj)).color(text_color),
                            ).sense(egui::Sense::click()))
                            .clicked()
                        {
                            detail.toggle(session_id);
                        }

                        // Model
                        let model_short = if s.model.contains("opus") {
                            "Opus"
                        } else if s.model.contains("sonnet") {
                            "Sonnet"
                        } else if s.model.contains("haiku") {
                            "Haiku"
                        } else {
                            &s.model
                        };
                        ui.colored_label(text_color, model_short);

                        // Duration
                        let mins = s.duration_minutes();
                        let dur = if mins >= 60 {
                            format!("{}h{}m", mins / 60, mins % 60)
                        } else {
                            format!("{}m", mins)
                        };
                        ui.colored_label(text_color, dur);

                        // Tokens
                        ui.colored_label(text_color, format_tokens(s.total_tokens()));

                        // Cost
                        let cost = settings.estimate_cost(
                            &s.model,
                            s.input_tokens,
                            s.output_tokens,
                            s.cache_creation_tokens,
                            s.cache_read_tokens,
                        );
                        ui.colored_label(text_color, format!("${:.2}", cost));

                        // Branch
                        let branch_label = if s.branch.is_empty() {
                            "-".to_string()
                        } else if s.branch.len() > 18 {
                            format!("{}...", &s.branch[..15])
                        } else {
                            s.branch.clone()
                        };
                        ui.colored_label(text_color, branch_label);

                        ui.end_row();

                        // Expanded timeline detail
                        if is_expanded && !detail.timeline_events.is_empty() {
                            // Span all 6 columns
                            ui.label(""); // col 1 padding
                            ui.end_row();
                            timeline::render(ui, &detail.timeline_events, session_id);
                            ui.end_row();
                        }
                    }
                });
        });
}
