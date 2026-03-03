use std::cmp::Reverse;

use chrono::{Duration, Utc};

use crate::parser;
use crate::settings::Settings;
use crate::types::{BranchMetrics, MessageRecord, MetricsState, ProjectMetrics, SessionMetrics};

impl MetricsState {
    /// Ingest a batch of new message records.
    /// Does NOT prune the burn window — call `prune_burn_window()` on the main thread.
    pub fn ingest(&mut self, records: &[MessageRecord]) {
        let today = Utc::now().date_naive();

        for rec in records {
            // Only count today's records
            if rec.timestamp.date_naive() != today {
                continue;
            }

            self.total_messages += 1;
            self.total_input += rec.input_tokens;
            self.total_output += rec.output_tokens;
            self.total_cache_creation += rec.cache_creation_tokens;
            self.total_cache_read += rec.cache_read_tokens;

            // Update last_updated
            match self.last_updated {
                Some(prev) if rec.timestamp > prev => self.last_updated = Some(rec.timestamp),
                None => self.last_updated = Some(rec.timestamp),
                _ => {}
            }

            // Per-session
            let session = self
                .sessions
                .entry(rec.session_id.clone())
                .or_insert_with(|| SessionMetrics {
                    project: parser::short_project_name(&rec.cwd),
                    model: rec.model.clone(),
                    first_seen: rec.timestamp,
                    last_seen: rec.timestamp,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 0,
                    message_count: 0,
                    branch: String::new(),
                });
            session.input_tokens += rec.input_tokens;
            session.output_tokens += rec.output_tokens;
            session.cache_creation_tokens += rec.cache_creation_tokens;
            session.cache_read_tokens += rec.cache_read_tokens;
            session.message_count += 1;
            if rec.timestamp > session.last_seen {
                session.last_seen = rec.timestamp;
            }
            if rec.timestamp < session.first_seen {
                session.first_seen = rec.timestamp;
            }
            // Use the latest model seen
            if !rec.model.is_empty() && rec.model != "unknown" {
                session.model = rec.model.clone();
            }
            // Use the latest non-empty branch
            if !rec.git_branch.is_empty() {
                session.branch = rec.git_branch.clone();
            }

            // Per-tool
            for tool_name in &rec.tool_names {
                *self.tools.entry(tool_name.clone()).or_insert(0) += 1;
            }

            // Per-branch
            if !rec.git_branch.is_empty() {
                let branch = self
                    .branches
                    .entry(rec.git_branch.clone())
                    .or_insert_with(|| BranchMetrics {
                        name: rec.git_branch.clone(),
                        ..Default::default()
                    });
                branch.input_tokens += rec.input_tokens;
                branch.output_tokens += rec.output_tokens;
                branch.cache_creation_tokens += rec.cache_creation_tokens;
                branch.cache_read_tokens += rec.cache_read_tokens;
                branch.message_count += 1;
            }

            // Burn window
            self.burn_window
                .push_back((rec.timestamp, rec.output_tokens));

            // Per-project
            let project_name = parser::short_project_name(&rec.cwd);
            let project = self
                .projects
                .entry(project_name.clone())
                .or_insert_with(|| ProjectMetrics {
                    name: project_name,
                    ..Default::default()
                });
            project.input_tokens += rec.input_tokens;
            project.output_tokens += rec.output_tokens;
            project.cache_creation_tokens += rec.cache_creation_tokens;
            project.cache_read_tokens += rec.cache_read_tokens;
            // Count unique sessions per project (increment only on first record for session)
            // This is approximate — fine for a widget
            if session.message_count == 1 {
                project.session_count += 1;
            }

            // Per-model
            let model_key = friendly_model_name(&rec.model);
            let model_metrics = self.models.entry(model_key.to_string()).or_default();
            model_metrics.input_tokens += rec.input_tokens;
            model_metrics.output_tokens += rec.output_tokens;
            model_metrics.cache_creation_tokens += rec.cache_creation_tokens;
            model_metrics.cache_read_tokens += rec.cache_read_tokens;
            model_metrics.message_count += 1;
        }
    }

    /// Prune burn window entries older than `window_minutes`. Call on the main thread.
    pub fn prune_burn_window(&mut self, window_minutes: i64) {
        let cutoff = Utc::now() - Duration::minutes(window_minutes);
        while self
            .burn_window
            .front()
            .is_some_and(|(ts, _)| *ts < cutoff)
        {
            self.burn_window.pop_front();
        }
    }

    /// Ingest records without the today-only filter (for backfill).
    #[allow(dead_code)]
    pub fn ingest_all(&mut self, records: &[MessageRecord]) {
        for rec in records {
            self.total_messages += 1;
            self.total_input += rec.input_tokens;
            self.total_output += rec.output_tokens;
            self.total_cache_creation += rec.cache_creation_tokens;
            self.total_cache_read += rec.cache_read_tokens;

            match self.last_updated {
                Some(prev) if rec.timestamp > prev => self.last_updated = Some(rec.timestamp),
                None => self.last_updated = Some(rec.timestamp),
                _ => {}
            }
        }
    }

    /// Total estimated cost across all models today.
    pub fn estimated_cost(&self, settings: &Settings) -> f64 {
        let mut cost = 0.0;
        for (model_name, m) in &self.models {
            cost += settings.estimate_cost(
                model_name,
                m.input_tokens,
                m.output_tokens,
                m.cache_creation_tokens,
                m.cache_read_tokens,
            );
        }
        cost
    }

    /// Number of sessions active within the threshold.
    pub fn active_session_count(&self, settings: &Settings) -> usize {
        let now = Utc::now();
        self.sessions
            .values()
            .filter(|s| s.is_active(now, settings.active_session_threshold_minutes))
            .count()
    }

    /// Sessions sorted by last_seen descending.
    pub fn sessions_sorted(&self) -> Vec<&SessionMetrics> {
        let mut sessions: Vec<_> = self.sessions.values().collect();
        sessions.sort_by_key(|s| Reverse(s.last_seen));
        sessions
    }

    /// Projects sorted by total tokens descending.
    pub fn projects_sorted(&self) -> Vec<&ProjectMetrics> {
        let mut projects: Vec<_> = self.projects.values().collect();
        projects.sort_by_key(|p| Reverse(p.total_tokens()));
        projects
    }

    /// Tools sorted by invocation count descending.
    pub fn tools_sorted(&self) -> Vec<(&String, &u64)> {
        let mut tools: Vec<_> = self.tools.iter().collect();
        tools.sort_by_key(|(_, count)| Reverse(*count));
        tools
    }

    /// Branches sorted by total tokens descending.
    pub fn branches_sorted(&self) -> Vec<&BranchMetrics> {
        let mut branches: Vec<_> = self.branches.values().collect();
        branches.sort_by_key(|b| Reverse(b.total_tokens()));
        branches
    }

    /// Output tokens per minute over the burn window.
    pub fn burn_rate_per_minute(&self, settings: &Settings) -> f64 {
        if self.burn_window.is_empty() {
            return 0.0;
        }
        let total: u64 = self.burn_window.iter().map(|(_, tokens)| tokens).sum();
        let window_minutes = settings.burn_rate_window_minutes as f64;
        total as f64 / window_minutes
    }
}

fn friendly_model_name(model: &str) -> &str {
    if model.contains("opus") {
        "opus"
    } else if model.contains("sonnet") {
        "sonnet"
    } else if model.contains("haiku") {
        "haiku"
    } else {
        model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use crate::types::{MessageRecord, MetricsState};
    use chrono::Utc;

    fn make_record(session: &str, model: &str, input: u64, output: u64) -> MessageRecord {
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        MessageRecord {
            session_id: session.to_string(),
            timestamp: Utc::now(),
            cwd: format!("{}/test-project", home),
            model: model.to_string(),
            input_tokens: input,
            output_tokens: output,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            tool_names: vec![],
            git_branch: String::new(),
        }
    }

    #[test]
    fn test_ingest_counts_tokens() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record("s1", "claude-sonnet-4-5", 100, 200),
            make_record("s1", "claude-sonnet-4-5", 150, 250),
        ];
        state.ingest(&records);

        assert_eq!(state.total_input, 250);
        assert_eq!(state.total_output, 450);
        assert_eq!(state.total_messages, 2);
    }

    #[test]
    fn test_ingest_separates_sessions() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record("s1", "claude-sonnet-4-5", 100, 200),
            make_record("s2", "claude-opus-4-5", 300, 400),
        ];
        state.ingest(&records);

        assert_eq!(state.sessions.len(), 2);
        assert_eq!(state.sessions["s1"].input_tokens, 100);
        assert_eq!(state.sessions["s2"].input_tokens, 300);
    }

    #[test]
    fn test_ingest_groups_by_model() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record("s1", "claude-sonnet-4-5", 100, 200),
            make_record("s2", "claude-opus-4-5", 300, 400),
        ];
        state.ingest(&records);

        assert_eq!(state.models.len(), 2);
        assert!(state.models.contains_key("sonnet"));
        assert!(state.models.contains_key("opus"));
    }

    #[test]
    fn test_estimated_cost_nonzero() {
        let mut state = MetricsState::default();
        let records = vec![make_record("s1", "claude-sonnet-4-5", 1_000_000, 500_000)];
        state.ingest(&records);

        let cost = state.estimated_cost(&Settings::default());
        assert!(cost > 0.0, "cost should be positive, got {}", cost);
    }

    #[test]
    fn test_friendly_model_name() {
        assert_eq!(friendly_model_name("claude-opus-4-5"), "opus");
        assert_eq!(friendly_model_name("claude-sonnet-4-5"), "sonnet");
        assert_eq!(friendly_model_name("claude-haiku-4-5"), "haiku");
        assert_eq!(friendly_model_name("some-unknown-model"), "some-unknown-model");
    }

    #[test]
    fn test_ingest_skips_yesterday() {
        let mut state = MetricsState::default();
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        let yesterday = Utc::now() - chrono::Duration::days(1);
        let records = vec![MessageRecord {
            session_id: "old".to_string(),
            timestamp: yesterday,
            cwd: format!("{}/old-proj", home),
            model: "claude-sonnet-4-5".to_string(),
            input_tokens: 500,
            output_tokens: 500,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            tool_names: vec![],
            git_branch: String::new(),
        }];
        state.ingest(&records);

        assert_eq!(state.total_input, 0);
        assert_eq!(state.total_output, 0);
        assert_eq!(state.total_messages, 0);
    }

    #[test]
    fn test_ingest_updates_last_updated() {
        let mut state = MetricsState::default();
        let records = vec![make_record("s1", "claude-sonnet-4-5", 100, 200)];
        let ts = records[0].timestamp;
        state.ingest(&records);

        assert_eq!(state.last_updated, Some(ts));
    }

    #[test]
    fn test_ingest_session_model_upgrade() {
        let mut state = MetricsState::default();

        // First record with "unknown" model
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        let rec1 = MessageRecord {
            session_id: "s1".to_string(),
            timestamp: Utc::now(),
            cwd: format!("{}/test-project", home),
            model: "unknown".to_string(),
            input_tokens: 10,
            output_tokens: 20,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            tool_names: vec![],
            git_branch: String::new(),
        };
        state.ingest(&[rec1]);
        assert_eq!(state.sessions["s1"].model, "unknown");

        // Second record with real model
        let rec2 = MessageRecord {
            session_id: "s1".to_string(),
            timestamp: Utc::now(),
            cwd: format!("{}/test-project", home),
            model: "claude-sonnet-4-5".to_string(),
            input_tokens: 30,
            output_tokens: 40,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            tool_names: vec![],
            git_branch: String::new(),
        };
        state.ingest(&[rec2]);
        assert_eq!(state.sessions["s1"].model, "claude-sonnet-4-5");
    }

    #[test]
    fn test_sessions_sorted_order() {
        let mut state = MetricsState::default();
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        let now = Utc::now();

        // Three sessions with staggered timestamps (oldest first in ingestion)
        let records = vec![
            MessageRecord {
                session_id: "oldest".to_string(),
                timestamp: now - chrono::Duration::minutes(30),
                cwd: format!("{}/proj", home),
                model: "sonnet".to_string(),
                input_tokens: 10,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
            MessageRecord {
                session_id: "middle".to_string(),
                timestamp: now - chrono::Duration::minutes(15),
                cwd: format!("{}/proj", home),
                model: "sonnet".to_string(),
                input_tokens: 20,
                output_tokens: 20,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
            MessageRecord {
                session_id: "newest".to_string(),
                timestamp: now,
                cwd: format!("{}/proj", home),
                model: "sonnet".to_string(),
                input_tokens: 30,
                output_tokens: 30,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
        ];
        state.ingest(&records);

        let sorted = state.sessions_sorted();
        assert_eq!(sorted.len(), 3);
        // Most recent first
        assert_eq!(sorted[0].input_tokens, 30); // newest
        assert_eq!(sorted[1].input_tokens, 20); // middle
        assert_eq!(sorted[2].input_tokens, 10); // oldest
    }

    #[test]
    fn test_projects_sorted_order() {
        let mut state = MetricsState::default();
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();

        // Two projects with different token counts
        let records = vec![
            MessageRecord {
                session_id: "s1".to_string(),
                timestamp: Utc::now(),
                cwd: format!("{}/small-proj", home),
                model: "sonnet".to_string(),
                input_tokens: 100,
                output_tokens: 100,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
            MessageRecord {
                session_id: "s2".to_string(),
                timestamp: Utc::now(),
                cwd: format!("{}/big-proj", home),
                model: "sonnet".to_string(),
                input_tokens: 1000,
                output_tokens: 1000,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
        ];
        state.ingest(&records);

        let sorted = state.projects_sorted();
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].name, "big-proj"); // highest tokens first
        assert_eq!(sorted[1].name, "small-proj");
    }

    #[test]
    fn test_active_session_count() {
        let mut state = MetricsState::default();
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        let now = Utc::now();

        let records = vec![
            // Active — timestamp is now
            MessageRecord {
                session_id: "active".to_string(),
                timestamp: now,
                cwd: format!("{}/proj", home),
                model: "sonnet".to_string(),
                input_tokens: 10,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
            // Stale — 30 minutes ago
            MessageRecord {
                session_id: "stale".to_string(),
                timestamp: now - chrono::Duration::minutes(30),
                cwd: format!("{}/proj", home),
                model: "sonnet".to_string(),
                input_tokens: 10,
                output_tokens: 10,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                tool_names: vec![],
                git_branch: String::new(),
            },
        ];
        state.ingest(&records);

        assert_eq!(state.active_session_count(&Settings::default()), 1);
    }

    // ── New Phase 2 tests ────────────────────────────────

    fn make_record_with_tools(
        session: &str,
        model: &str,
        input: u64,
        output: u64,
        tools: Vec<&str>,
        branch: &str,
    ) -> MessageRecord {
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        MessageRecord {
            session_id: session.to_string(),
            timestamp: Utc::now(),
            cwd: format!("{}/test-project", home),
            model: model.to_string(),
            input_tokens: input,
            output_tokens: output,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            tool_names: tools.into_iter().map(String::from).collect(),
            git_branch: branch.to_string(),
        }
    }

    #[test]
    fn test_ingest_accumulates_tools() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record_with_tools("s1", "sonnet", 10, 20, vec!["Bash", "Read"], ""),
            make_record_with_tools("s1", "sonnet", 10, 20, vec!["Bash", "Write"], ""),
        ];
        state.ingest(&records);

        assert_eq!(state.tools["Bash"], 2);
        assert_eq!(state.tools["Read"], 1);
        assert_eq!(state.tools["Write"], 1);
    }

    #[test]
    fn test_tools_sorted_order() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record_with_tools("s1", "sonnet", 10, 20, vec!["Bash", "Read", "Bash"], ""),
            make_record_with_tools("s1", "sonnet", 10, 20, vec!["Write"], ""),
        ];
        state.ingest(&records);

        let sorted = state.tools_sorted();
        assert_eq!(*sorted[0].0, "Bash");
        assert_eq!(*sorted[0].1, 2);
    }

    #[test]
    fn test_ingest_accumulates_branches() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record_with_tools("s1", "sonnet", 100, 200, vec![], "main"),
            make_record_with_tools("s2", "sonnet", 300, 400, vec![], "feature/auth"),
        ];
        state.ingest(&records);

        assert_eq!(state.branches.len(), 2);
        assert_eq!(state.branches["main"].input_tokens, 100);
        assert_eq!(state.branches["main"].output_tokens, 200);
        assert_eq!(state.branches["feature/auth"].input_tokens, 300);
        assert_eq!(state.branches["feature/auth"].output_tokens, 400);
    }

    #[test]
    fn test_branches_sorted_order() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record_with_tools("s1", "sonnet", 100, 100, vec![], "small-branch"),
            make_record_with_tools("s2", "sonnet", 1000, 1000, vec![], "big-branch"),
        ];
        state.ingest(&records);

        let sorted = state.branches_sorted();
        assert_eq!(sorted[0].name, "big-branch");
        assert_eq!(sorted[1].name, "small-branch");
    }

    #[test]
    fn test_session_branch_uses_latest() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record_with_tools("s1", "sonnet", 10, 20, vec![], "old-branch"),
            make_record_with_tools("s1", "sonnet", 10, 20, vec![], "new-branch"),
        ];
        state.ingest(&records);

        assert_eq!(state.sessions["s1"].branch, "new-branch");
    }

    #[test]
    fn test_empty_branch_not_tracked() {
        let mut state = MetricsState::default();
        let records = vec![make_record_with_tools("s1", "sonnet", 10, 20, vec![], "")];
        state.ingest(&records);

        assert!(state.branches.is_empty());
    }

    #[test]
    fn test_burn_window_populated() {
        let mut state = MetricsState::default();
        let records = vec![make_record_with_tools("s1", "sonnet", 10, 200, vec![], "")];
        state.ingest(&records);

        assert_eq!(state.burn_window.len(), 1);
        assert_eq!(state.burn_window[0].1, 200);
    }

    #[test]
    fn test_burn_rate_per_minute() {
        let mut state = MetricsState::default();
        let now = Utc::now();
        // Manually push entries into the window
        state.burn_window.push_back((now, 5000));
        state.burn_window.push_back((now, 5000));

        let rate = state.burn_rate_per_minute(&Settings::default());
        // 10000 tokens over burn_rate_window_minutes (10) = 1000 tok/min
        assert!((rate - 1000.0).abs() < 0.1);
    }

    #[test]
    fn test_burn_rate_empty_is_zero() {
        let state = MetricsState::default();
        assert_eq!(state.burn_rate_per_minute(&Settings::default()), 0.0);
    }

    #[test]
    fn test_no_tools_empty_map() {
        let mut state = MetricsState::default();
        let records = vec![
            make_record("s1", "sonnet", 10, 20),
            make_record("s2", "sonnet", 30, 40),
        ];
        state.ingest(&records);

        assert!(state.tools.is_empty());
    }
}
