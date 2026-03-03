use eframe::egui;

use crate::types::format_tokens;

/// A single event on the timeline.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: EventType,
    pub tokens: u64,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    User,
    Assistant,
    Tool,
}

impl EventType {
    fn color(&self) -> egui::Color32 {
        match self {
            Self::User => egui::Color32::from_rgb(100, 160, 255),     // blue
            Self::Assistant => egui::Color32::from_rgb(180, 130, 255), // purple
            Self::Tool => egui::Color32::from_rgb(150, 150, 150),      // gray
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

/// Parse a JSONL file and extract timeline events for a specific session.
pub fn load_session_timeline(
    projects_dir: &std::path::Path,
    session_id: &str,
) -> Vec<TimelineEvent> {
    let mut events = Vec::new();
    scan_for_session(projects_dir, session_id, &mut events);
    events.sort_by_key(|e| e.timestamp);
    events
}

fn scan_for_session(
    dir: &std::path::Path,
    session_id: &str,
    events: &mut Vec<TimelineEvent>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_for_session(&path, session_id, events);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    if let Some(evt) = parse_timeline_event(line, session_id) {
                        events.push(evt);
                    }
                }
            }
        }
    }
}

/// Parse a single JSONL line into a TimelineEvent if it belongs to the given session.
fn parse_timeline_event(line: &str, session_id: &str) -> Option<TimelineEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let v: serde_json::Value = serde_json::from_str(line).ok()?;

    // Check session
    if v.get("sessionId")?.as_str()? != session_id {
        return None;
    }

    let timestamp_str = v.get("timestamp")?.as_str()?;
    let timestamp = timestamp_str.parse().ok()?;

    let msg_type = v.get("type")?.as_str()?;

    let message = v.get("message")?;

    let (event_type, tokens, tool_name) = match msg_type {
        "user" => (EventType::User, 0, None),
        "assistant" => {
            let usage_tokens = message
                .get("usage")
                .and_then(|u| u.get("output_tokens"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            // Check for tool_use in content
            let tool = message
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| {
                            item.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                        })
                        .and_then(|item| item.get("name").and_then(|n| n.as_str()))
                        .map(String::from)
                });
            (EventType::Assistant, usage_tokens, tool)
        }
        "tool_result" | "tool" => (EventType::Tool, 0, None),
        _ => return None,
    };

    Some(TimelineEvent {
        timestamp,
        event_type,
        tokens,
        tool_name,
    })
}

/// Render a session timeline detail panel.
pub fn render(
    ui: &mut egui::Ui,
    events: &[TimelineEvent],
    session_id: &str,
) {
    if events.is_empty() {
        ui.colored_label(
            egui::Color32::from_rgb(120, 120, 120),
            format!("No events found for session {}", &session_id[..8.min(session_id.len())]),
        );
        return;
    }

    let first_ts = events.first().unwrap().timestamp;
    let last_ts = events.last().unwrap().timestamp;
    let span_secs = (last_ts - first_ts).num_seconds().max(1) as f64;

    let timeline_width = ui.available_width() - 20.0;
    let timeline_height = 32.0;

    // Timeline header
    ui.horizontal(|ui| {
        ui.strong(format!(
            "Session Timeline ({} events)",
            events.len()
        ));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let duration_mins = span_secs / 60.0;
            if duration_mins >= 60.0 {
                ui.label(format!("{:.0}h {:.0}m", duration_mins / 60.0, duration_mins % 60.0));
            } else {
                ui.label(format!("{:.0}m", duration_mins));
            }
        });
    });

    ui.add_space(2.0);

    // Draw timeline bar
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(timeline_width, timeline_height), egui::Sense::hover());

    let painter = ui.painter();

    // Background
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 30, 40));

    // Draw each event as a dot/line
    let dot_radius = 4.0;
    let mut hover_info = None;

    for evt in events {
        let offset_secs = (evt.timestamp - first_ts).num_seconds() as f64;
        let x = rect.left() + (offset_secs / span_secs) as f32 * timeline_width;
        let y = match evt.event_type {
            EventType::User => rect.top() + 8.0,
            EventType::Assistant => rect.center().y,
            EventType::Tool => rect.bottom() - 8.0,
        };

        let center = egui::pos2(x, y);
        painter.circle_filled(center, dot_radius, evt.event_type.color());

        // Check hover
        if let Some(hover_pos) = response.hover_pos() {
            let dist = (hover_pos - center).length();
            if dist < dot_radius + 3.0 {
                hover_info = Some(evt.clone());
            }
        }
    }

    // Row labels on the right
    ui.horizontal(|ui| {
        ui.colored_label(EventType::User.color(), "user");
        ui.label(" | ");
        ui.colored_label(EventType::Assistant.color(), "assistant");
        ui.label(" | ");
        ui.colored_label(EventType::Tool.color(), "tool");
    });

    // Tooltip on hover
    if let Some(evt) = hover_info {
        egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("timeline_tip"), |ui| {
            ui.label(format!(
                "{} {}",
                evt.timestamp.format("%H:%M:%S"),
                evt.event_type.label()
            ));
            if evt.tokens > 0 {
                ui.label(format!("{} tokens", format_tokens(evt.tokens)));
            }
            if let Some(tool) = &evt.tool_name {
                ui.label(format!("tool: {}", tool));
            }
        });
    }
}
