//! Tunables, with a config file layered over compiled-in defaults.
//!
//! The file keys stay camelCase (`waitBonusPerSec`, not `wait_bonus_per_sec`) so a
//! config.json written for the TypeScript version keeps working unchanged.

use crate::json;

/// Base weight per status — higher means "needs you sooner".
#[derive(Debug, Clone, PartialEq)]
pub struct Weights {
    pub blocked: f64,
    pub done: f64,
    pub working: f64,
    pub idle: f64,
    /// Used for any status herdr reports that we don't know about.
    pub default: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub weights: Weights,
    /// Extra priority per second a blocked agent has been waiting.
    pub wait_bonus_per_sec: f64,
    /// Cap on the wait bonus so one very old block doesn't dominate forever.
    pub max_wait_bonus: f64,
    /// Extra priority per second a done agent sits unreviewed.
    pub done_bonus_per_sec: f64,
    /// Cap on the done bonus. Keep it below (blocked − done) so blocked stays on top.
    pub max_done_bonus: f64,
    /// How often to refresh, in milliseconds.
    pub poll_interval_ms: u64,
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            blocked: 1000.0,
            done: 500.0,
            working: 100.0,
            idle: 10.0,
            default: 50.0,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            weights: Weights::default(),
            wait_bonus_per_sec: 1.0,
            max_wait_bonus: 600.0, // 10 minutes' worth
            done_bonus_per_sec: 0.5,
            max_done_bonus: 100.0, // done tops out at 600, still below the blocked base of 1000
            poll_interval_ms: 1500,
        }
    }
}

/// Directory holding config.json: the one herdr hands plugins, else the standalone path.
pub fn config_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("HERDR_PLUGIN_CONFIG_DIR") {
        return std::path::PathBuf::from(dir);
    }
    home_dir().join(".config").join("herdr-triage")
}

pub fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Load config.json from the config dir, falling back to defaults.
///
/// An unreadable file is "no config"; a malformed one is worth saying out loud, since
/// silently ignoring it looks identical to the edit having no effect.
pub fn load_config() -> Config {
    let path = config_dir().join("config.json");
    match std::fs::read_to_string(&path) {
        Ok(raw) => match merge(&raw) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("triage: ignoring {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}

/// Apply a config file's fields over the defaults. Every field is optional; anything
/// absent, null, or of the wrong type keeps its default.
pub fn merge(raw: &str) -> Result<Config, String> {
    let v = json::parse(raw)?;
    let mut cfg = Config::default();

    if let Some(w) = v.get("weights") {
        let f = |key: &str, cur: f64| w.num_field(key).unwrap_or(cur);
        cfg.weights = Weights {
            blocked: f("blocked", cfg.weights.blocked),
            done: f("done", cfg.weights.done),
            working: f("working", cfg.weights.working),
            idle: f("idle", cfg.weights.idle),
            default: f("default", cfg.weights.default),
        };
    }
    cfg.wait_bonus_per_sec = v
        .num_field("waitBonusPerSec")
        .unwrap_or(cfg.wait_bonus_per_sec);
    cfg.max_wait_bonus = v.num_field("maxWaitBonus").unwrap_or(cfg.max_wait_bonus);
    cfg.done_bonus_per_sec = v
        .num_field("doneBonusPerSec")
        .unwrap_or(cfg.done_bonus_per_sec);
    cfg.max_done_bonus = v.num_field("maxDoneBonus").unwrap_or(cfg.max_done_bonus);
    if let Some(ms) = v.num_field("pollIntervalMs") {
        // A zero or negative interval would spin the socket as fast as the loop runs.
        cfg.poll_interval_ms = if ms >= 1.0 { ms as u64 } else { 1 };
    }
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_yields_defaults() {
        assert_eq!(merge("{}").unwrap(), Config::default());
    }

    #[test]
    fn merges_weights_over_the_defaults() {
        let c = merge(r#"{"weights":{"blocked":2000}}"#).unwrap();
        assert_eq!(c.weights.blocked, 2000.0);
        // Other weights are preserved rather than zeroed by the partial object.
        assert_eq!(c.weights.done, Weights::default().done);
        assert_eq!(c.weights.idle, Weights::default().idle);
    }

    #[test]
    fn reads_the_example_config_fields() {
        let c = merge(
            r#"{"waitBonusPerSec":2,"maxWaitBonus":900,"doneBonusPerSec":0.25,
                "maxDoneBonus":50,"pollIntervalMs":3000}"#,
        )
        .unwrap();
        assert_eq!(c.wait_bonus_per_sec, 2.0);
        assert_eq!(c.max_wait_bonus, 900.0);
        assert_eq!(c.done_bonus_per_sec, 0.25);
        assert_eq!(c.max_done_bonus, 50.0);
        assert_eq!(c.poll_interval_ms, 3000);
    }

    #[test]
    fn a_zero_poll_interval_does_not_become_a_busy_loop() {
        assert_eq!(
            merge(r#"{"pollIntervalMs":0}"#).unwrap().poll_interval_ms,
            1
        );
    }

    #[test]
    fn malformed_config_is_an_error_not_a_panic() {
        assert!(merge("{not json").is_err());
    }
}
