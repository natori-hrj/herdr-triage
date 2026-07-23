//! Rendering the ranked list. Pure — no I/O, so it's testable as plain string work.

use crate::priority::Ranked;

const LABEL_WIDTH: usize = 32;

fn glyph(status: &str) -> &'static str {
    match status {
        "blocked" => "🔴",
        "done" => "✅",
        "working" => "⚙️",
        "idle" => "💤",
        _ => "·",
    }
}

/// Human-friendly duration: "42s", "5m", "1h3m".
pub fn format_duration(ms: u64) -> String {
    let s = ms / 1000;
    if s < 60 {
        return format!("{}s", s);
    }
    let m = s / 60;
    if m < 60 {
        return format!("{}m", m);
    }
    format!("{}h{}m", m / 60, m % 60)
}

/// Truncate to `n` characters, with an ellipsis standing in for the last one.
///
/// Counts characters, not bytes: agent titles carry non-ASCII often enough that slicing by
/// byte would panic on a multi-byte boundary.
fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// What to call this agent in the list: its task if herdr knows one, else the pane it's in.
fn label(r: &Ranked) -> String {
    let task = r.item.task.as_deref().unwrap_or("").trim();
    if !task.is_empty() {
        return trunc(task, LABEL_WIDTH);
    }
    let fallback = r.item.pane_id.as_deref().unwrap_or(&r.item.terminal_id);
    trunc(fallback, LABEL_WIDTH)
}

/// Render the ranked agents as a compact, glanceable triage list.
pub fn render_triage(ranked: &[Ranked]) -> String {
    if ranked.is_empty() {
        return "No agents.".to_string();
    }
    let mut lines = vec![format!("Attention triage — {} agent(s)", ranked.len())];
    for r in ranked {
        // A wait time is only meaningful for blocked agents — the others show a dash so the
        // column stays aligned.
        let wait = if r.item.status == "blocked" {
            format_duration(r.item.in_status_ms)
        } else {
            "—".to_string()
        };
        let pane = r.item.pane_id.as_deref().unwrap_or(&r.item.terminal_id);
        lines.push(format!(
            "{} {:>5}  {:<width$}  {}",
            glyph(&r.item.status),
            wait,
            label(r),
            pane,
            width = LABEL_WIDTH
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::RankItem;

    fn ranked(status: &str, in_status_ms: u64, pane: Option<&str>, task: Option<&str>) -> Ranked {
        Ranked {
            item: RankItem {
                terminal_id: "term-1".to_string(),
                agent: Some("claude".to_string()),
                pane_id: pane.map(str::to_string),
                task: task.map(str::to_string),
                cwd: Some("/x".to_string()),
                status: status.to_string(),
                in_status_ms,
            },
            score: 1300.0,
        }
    }

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration(42_000), "42s");
        assert_eq!(format_duration(5 * 60_000), "5m");
        assert_eq!(format_duration(63 * 60_000), "1h3m");
    }

    #[test]
    fn says_so_when_there_are_no_agents() {
        assert_eq!(render_triage(&[]), "No agents.");
    }

    #[test]
    fn shows_a_wait_time_for_blocked_agents_and_a_dash_otherwise() {
        let out = render_triage(&[
            ranked("blocked", 300_000, Some("w1:p1"), Some("Fix login")),
            ranked("done", 10_000, Some("w1:p2"), Some("Fix login")),
        ]);
        assert!(out.contains("🔴"));
        assert!(out.contains("5m"));
        assert!(out.contains("✅"));
        assert!(out.contains("w1:p2"));
        assert!(out.contains("—"));
        // The done agent's 10s in status is not shown as a wait.
        assert!(!out.contains("10s"));
    }

    #[test]
    fn includes_a_header_with_the_agent_count() {
        let out = render_triage(&[
            ranked("blocked", 0, Some("w1:p1"), Some("a")),
            ranked("idle", 0, Some("w1:p2"), Some("b")),
        ]);
        assert!(out.contains("2 agent(s)"));
    }

    #[test]
    fn falls_back_to_the_pane_id_when_there_is_no_task() {
        let out = render_triage(&[ranked("idle", 0, Some("w1:pB"), None)]);
        assert!(out.contains("w1:pB"));
        let blank = render_triage(&[ranked("idle", 0, Some("w1:pB"), Some("   "))]);
        assert!(blank.contains("w1:pB"));
    }

    #[test]
    fn falls_back_to_the_terminal_id_when_there_is_no_pane_either() {
        let out = render_triage(&[ranked("idle", 0, None, None)]);
        assert!(out.contains("term-1"));
    }

    #[test]
    fn truncates_a_long_title_on_a_character_boundary() {
        let long = "認証モジュールを移行してテストを全部書き直すタスクです。長いタイトル";
        let out = render_triage(&[ranked("blocked", 0, Some("w1:p1"), Some(long))]);
        assert!(out.contains('…'));
        assert!(out.contains("認証モジュール"));
    }

    #[test]
    fn an_unknown_status_still_renders_a_row() {
        let out = render_triage(&[ranked("frobnicating", 0, Some("w1:p1"), Some("x"))]);
        assert!(out.contains('·'));
        assert!(out.contains("w1:p1"));
    }
}
