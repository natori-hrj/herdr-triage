//! Attention priority: who needs you first.

use crate::config::Config;
use crate::herdr::Agent;

#[derive(Debug, Clone, PartialEq)]
pub struct RankItem {
    pub terminal_id: String,
    pub agent: Option<String>,
    pub pane_id: Option<String>,
    pub task: Option<String>,
    pub cwd: Option<String>,
    pub status: String,
    /// How long the agent has been in its current status (ms).
    pub in_status_ms: u64,
}

impl RankItem {
    pub fn from_agent(a: &Agent, in_status_ms: u64) -> Self {
        RankItem {
            terminal_id: a.terminal_id.clone(),
            agent: a.agent.clone(),
            pane_id: a.pane_id.clone(),
            task: a.task.clone(),
            cwd: a.cwd.clone(),
            status: a.status.clone(),
            in_status_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub item: RankItem,
    pub score: f64,
}

/// Priority score for an agent. Blocked agents rank highest and rise further the longer
/// they've been waiting (capped); done agents (need review / a next task) come next;
/// working and idle are low. Pure — trivially unit-testable.
pub fn score(status: &str, in_status_ms: u64, cfg: &Config) -> f64 {
    let secs = in_status_ms as f64 / 1000.0;
    match status {
        "blocked" => cfg.weights.blocked + (secs * cfg.wait_bonus_per_sec).min(cfg.max_wait_bonus),
        // A done agent that's sat unreviewed nudges up so it isn't buried under fresh done
        // agents — capped so it never overtakes a blocked agent.
        "done" => cfg.weights.done + (secs * cfg.done_bonus_per_sec).min(cfg.max_done_bonus),
        "working" => cfg.weights.working,
        "idle" => cfg.weights.idle,
        _ => cfg.weights.default,
    }
}

/// Rank agents by attention priority, highest first. Ties break on longer time-in-status.
pub fn rank(items: Vec<RankItem>, cfg: &Config) -> Vec<Ranked> {
    let mut ranked: Vec<Ranked> = items
        .into_iter()
        .map(|item| {
            let score = score(&item.status, item.in_status_ms, cfg);
            Ranked { item, score }
        })
        .collect();
    ranked.sort_by(|a, b| {
        // partial_cmp can only be None for a NaN score, which needs a NaN weight in the
        // config to happen at all; treat it as equal and let the tie-break decide.
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.item.in_status_ms.cmp(&a.item.in_status_ms))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, status: &str, in_status_ms: u64) -> RankItem {
        RankItem {
            terminal_id: id.to_string(),
            agent: Some("claude".to_string()),
            pane_id: Some("w1:p1".to_string()),
            task: Some("task".to_string()),
            cwd: Some("/x".to_string()),
            status: status.to_string(),
            in_status_ms,
        }
    }

    #[test]
    fn orders_blocked_done_working_idle_at_equal_time() {
        let c = Config::default();
        assert!(score("blocked", 0, &c) > score("done", 0, &c));
        assert!(score("done", 0, &c) > score("working", 0, &c));
        assert!(score("working", 0, &c) > score("idle", 0, &c));
    }

    #[test]
    fn raises_a_blocked_agents_score_the_longer_it_waits() {
        let c = Config::default();
        assert!(score("blocked", 60_000, &c) > score("blocked", 0, &c));
    }

    #[test]
    fn caps_the_wait_bonus() {
        let c = Config::default();
        assert_eq!(
            score("blocked", u64::MAX, &c),
            c.weights.blocked + c.max_wait_bonus
        );
    }

    #[test]
    fn does_not_apply_a_time_bonus_to_working_or_idle() {
        let c = Config::default();
        assert_eq!(score("working", 10u64.pow(9), &c), c.weights.working);
        assert_eq!(score("idle", 10u64.pow(9), &c), c.weights.idle);
    }

    #[test]
    fn raises_a_done_agents_score_the_longer_it_sits_capped() {
        let c = Config::default();
        assert!(score("done", 60_000, &c) > score("done", 0, &c));
        assert_eq!(
            score("done", u64::MAX, &c),
            c.weights.done + c.max_done_bonus
        );
    }

    #[test]
    fn keeps_a_maxed_out_done_agent_below_a_just_blocked_agent() {
        let c = Config::default();
        assert!(score("done", u64::MAX, &c) < score("blocked", 0, &c));
    }

    #[test]
    fn uses_the_default_weight_for_unknown_statuses() {
        let c = Config::default();
        assert_eq!(score("frobnicating", 0, &c), c.weights.default);
        assert_eq!(score("unknown", 0, &c), c.weights.default);
    }

    #[test]
    fn puts_the_longest_blocked_agent_first() {
        let ranked = rank(
            vec![
                item("a", "blocked", 5_000),
                item("b", "blocked", 120_000),
                item("c", "done", 0),
                item("d", "idle", 0),
            ],
            &Config::default(),
        );
        let ids: Vec<&str> = ranked.iter().map(|r| r.item.terminal_id.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c", "d"]);
    }

    #[test]
    fn breaks_score_ties_by_longer_time_in_status() {
        let ranked = rank(
            vec![item("x", "working", 1_000), item("y", "working", 9_000)],
            &Config::default(),
        );
        assert_eq!(ranked[0].item.terminal_id, "y");
    }
}
