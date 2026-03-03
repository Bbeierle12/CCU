use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};

/// A parsed record from a JSONL assistant message with usage data.
#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub cwd: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub tool_names: Vec<String>,
    pub git_branch: String,
}

/// Aggregated metrics for a single session.
#[derive(Debug, Clone)]
pub struct SessionMetrics {
    pub project: String,
    pub model: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub message_count: u64,
    pub branch: String,
}

impl SessionMetrics {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }

    pub fn duration_minutes(&self) -> i64 {
        (self.last_seen - self.first_seen).num_minutes()
    }

    pub fn is_active(&self, now: DateTime<Utc>, threshold_minutes: i64) -> bool {
        (now - self.last_seen).num_minutes() < threshold_minutes
    }
}

/// Aggregated metrics for a project (decoded path).
#[derive(Debug, Clone, Default)]
pub struct ProjectMetrics {
    pub name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub session_count: u64,
}

impl ProjectMetrics {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }
}

/// Per-model token breakdown.
#[derive(Debug, Clone, Default)]
pub struct ModelMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub message_count: u64,
}

/// Per-branch token breakdown.
#[derive(Debug, Clone, Default)]
pub struct BranchMetrics {
    pub name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub message_count: u64,
}

impl BranchMetrics {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }
}

/// Format a token count for display: 1234 -> "1.2K", 1234567 -> "1.2M".
pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// The full metrics state, updated by the aggregator.
#[derive(Debug, Clone, Default)]
pub struct MetricsState {
    pub sessions: HashMap<String, SessionMetrics>,
    pub projects: HashMap<String, ProjectMetrics>,
    pub models: HashMap<String, ModelMetrics>,
    pub tools: HashMap<String, u64>,
    pub branches: HashMap<String, BranchMetrics>,
    pub burn_window: VecDeque<(DateTime<Utc>, u64)>,
    pub total_input: u64,
    pub total_output: u64,
    pub total_cache_creation: u64,
    pub total_cache_read: u64,
    pub total_messages: u64,
    pub last_updated: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens_small() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn test_format_tokens_thousands() {
        assert_eq!(format_tokens(1_000), "1.0K");
        assert_eq!(format_tokens(1_500), "1.5K");
        assert_eq!(format_tokens(999_999), "1000.0K");
    }

    #[test]
    fn test_format_tokens_millions() {
        assert_eq!(format_tokens(1_000_000), "1.0M");
        assert_eq!(format_tokens(2_500_000), "2.5M");
    }

    #[test]
    fn test_session_duration() {
        let s = SessionMetrics {
            project: "proj".into(),
            model: "sonnet".into(),
            first_seen: "2026-03-03T10:00:00Z".parse().unwrap(),
            last_seen: "2026-03-03T11:30:00Z".parse().unwrap(),
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            message_count: 0,
            branch: String::new(),
        };
        assert_eq!(s.duration_minutes(), 90);
        assert_eq!(s.total_tokens(), 0);
    }
}
