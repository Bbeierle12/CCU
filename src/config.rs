// NOTE: Tunable values (thresholds, pricing, window size) have moved to
// `settings::Settings` which persists to ~/.config/claude-usage-card/settings.json.
// Only path constants remain here.

/// Subdirectory under home where Claude Code stores projects.
pub const CLAUDE_PROJECTS_REL: &str = ".claude/projects";

/// Relative path under home for the SQLite database.
pub const DB_REL_PATH: &str = ".config/claude-usage-card/usage.db";
