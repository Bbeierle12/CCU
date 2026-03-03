use crate::types::MessageRecord;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Try to parse a single JSONL line into a MessageRecord.
/// Returns None for non-assistant messages, messages without usage, or malformed lines.
pub fn parse_line(line: &str) -> Option<MessageRecord> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let v: Value = serde_json::from_str(line).ok()?;

    // Only care about assistant messages (they have usage data)
    if v.get("type")?.as_str()? != "assistant" {
        return None;
    }

    let session_id = v.get("sessionId")?.as_str()?.to_string();
    let cwd = v.get("cwd").and_then(|c| c.as_str()).unwrap_or("unknown").to_string();
    let timestamp_str = v.get("timestamp")?.as_str()?;
    let timestamp: DateTime<Utc> = timestamp_str.parse().ok()?;
    let git_branch = v.get("gitBranch").and_then(|b| b.as_str()).unwrap_or("").to_string();

    let message = v.get("message")?;
    let model = message
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();

    let tool_names: Vec<String> = message
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
                .filter_map(|item| item.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let usage = message.get("usage")?;

    let input_tokens = usage
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_creation_tokens = usage
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read_tokens = usage
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Some(MessageRecord {
        session_id,
        timestamp,
        cwd,
        model,
        input_tokens,
        output_tokens,
        cache_creation_tokens,
        cache_read_tokens,
        tool_names,
        git_branch,
    })
}

/// Parse all lines from a buffer, returning only valid MessageRecords.
pub fn parse_buffer(buf: &str) -> Vec<MessageRecord> {
    buf.lines().filter_map(parse_line).collect()
}

/// Extract a short project name from a cwd path.
/// "/home/bbeierle12/Agent-Shell" -> "Agent-Shell"
/// "/home/bbeierle12" -> "~"
pub fn short_project_name(cwd: &str) -> String {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if cwd == home {
        return "~".to_string();
    }

    cwd.strip_prefix(&home)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(cwd)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_project_name() {
        let home = dirs::home_dir().unwrap().to_string_lossy().to_string();
        assert_eq!(short_project_name(&home), "~");
        assert_eq!(
            short_project_name(&format!("{}/Agent-Shell", home)),
            "Agent-Shell"
        );
    }

    #[test]
    fn test_parse_line_non_assistant() {
        let line = r#"{"type":"user","sessionId":"abc","timestamp":"2026-03-03T00:00:00Z","cwd":"/tmp","message":{"role":"user","content":"hi"}}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn test_parse_line_assistant_with_usage() {
        let line = r#"{"type":"assistant","sessionId":"abc-123","timestamp":"2026-03-03T10:30:00Z","cwd":"/home/user/proj","message":{"model":"claude-opus-4-6","role":"assistant","content":[],"usage":{"input_tokens":100,"output_tokens":200,"cache_creation_input_tokens":50,"cache_read_input_tokens":1000}}}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.session_id, "abc-123");
        assert_eq!(rec.model, "claude-opus-4-6");
        assert_eq!(rec.input_tokens, 100);
        assert_eq!(rec.output_tokens, 200);
        assert_eq!(rec.cache_creation_tokens, 50);
        assert_eq!(rec.cache_read_tokens, 1000);
    }

    #[test]
    fn test_parse_line_empty_string() {
        assert!(parse_line("").is_none());
    }

    #[test]
    fn test_parse_line_whitespace_only() {
        assert!(parse_line("   \n").is_none());
    }

    #[test]
    fn test_parse_line_invalid_json() {
        assert!(parse_line("{not json}").is_none());
    }

    #[test]
    fn test_parse_line_missing_usage() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[]}}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn test_parse_line_missing_session_id() {
        let line = r#"{"type":"assistant","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn test_parse_line_missing_timestamp() {
        let line = r#"{"type":"assistant","sessionId":"abc","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn test_parse_line_zero_tokens() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.input_tokens, 0);
        assert_eq!(rec.output_tokens, 0);
        assert_eq!(rec.cache_creation_tokens, 0);
        assert_eq!(rec.cache_read_tokens, 0);
    }

    #[test]
    fn test_parse_line_extracts_tool_names() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[{"type":"tool_use","name":"Bash","id":"t1","input":{}},{"type":"text","text":"hello"},{"type":"tool_use","name":"Read","id":"t2","input":{}}],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.tool_names, vec!["Bash", "Read"]);
    }

    #[test]
    fn test_parse_line_no_tool_use_empty_vec() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        let rec = parse_line(line).unwrap();
        assert!(rec.tool_names.is_empty());
    }

    #[test]
    fn test_parse_line_empty_content_array() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        let rec = parse_line(line).unwrap();
        assert!(rec.tool_names.is_empty());
    }

    #[test]
    fn test_parse_line_extracts_git_branch() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","gitBranch":"feature/auth","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.git_branch, "feature/auth");
    }

    #[test]
    fn test_parse_line_missing_git_branch_defaults_empty() {
        let line = r#"{"type":"assistant","sessionId":"abc","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":20}}}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.git_branch, "");
    }

    #[test]
    fn test_parse_buffer_mixed() {
        let buf = [
            "",                               // blank
            r#"{"type":"user","sessionId":"a","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"role":"user","content":"hi"}}"#, // user msg
            "{bad json}",                     // invalid
            r#"{"type":"assistant","sessionId":"a","timestamp":"2026-03-03T10:00:00Z","cwd":"/tmp","message":{"model":"sonnet","role":"assistant","content":[],"usage":{"input_tokens":10,"output_tokens":20}}}"#, // valid
            "   ",                            // whitespace
            r#"{"type":"assistant","sessionId":"b","timestamp":"2026-03-03T11:00:00Z","cwd":"/tmp","message":{"model":"opus","role":"assistant","content":[],"usage":{"input_tokens":30,"output_tokens":40}}}"#, // valid
        ].join("\n");

        let recs = parse_buffer(&buf);
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].session_id, "a");
        assert_eq!(recs[1].session_id, "b");
    }
}
