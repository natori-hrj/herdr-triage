//! How long each agent has been in its current status.
//!
//! herdr doesn't timestamp status changes, so we time them ourselves by diffing successive
//! `agent.list` snapshots. The clock is injected so the behavior is deterministic in tests.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::herdr::Agent;

struct Entry {
    status: String,
    since: u64,
}

pub struct AttentionTracker {
    entries: HashMap<String, Entry>,
    now: Box<dyn Fn() -> u64>,
}

/// Milliseconds since the Unix epoch.
fn wall_clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl AttentionTracker {
    pub fn new() -> Self {
        Self::with_clock(Box::new(wall_clock_ms))
    }

    pub fn with_clock(now: Box<dyn Fn() -> u64>) -> Self {
        AttentionTracker {
            entries: HashMap::new(),
            now,
        }
    }

    /// Record a fresh snapshot: reset the timer for any agent whose status changed, and
    /// forget agents that are no longer listed.
    pub fn observe(&mut self, agents: &[Agent]) {
        let t = (self.now)();
        for a in agents {
            let changed = match self.entries.get(&a.terminal_id) {
                Some(prev) => prev.status != a.status,
                None => true,
            };
            if changed {
                self.entries.insert(
                    a.terminal_id.clone(),
                    Entry {
                        status: a.status.clone(),
                        since: t,
                    },
                );
            }
        }
        self.entries
            .retain(|id, _| agents.iter().any(|a| &a.terminal_id == id));
    }

    /// Milliseconds the agent has been in its current status (0 if unknown).
    pub fn in_status_ms(&self, terminal_id: &str) -> u64 {
        match self.entries.get(terminal_id) {
            // saturating_sub, not a subtraction: a wall clock can step backwards, and a
            // negative age would wrap into a huge one and pin the agent to the top.
            Some(e) => (self.now)().saturating_sub(e.since),
            None => 0,
        }
    }
}

impl Default for AttentionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn agent(id: &str, status: &str) -> Agent {
        Agent {
            terminal_id: id.to_string(),
            agent: Some("claude".to_string()),
            task: Some("task".to_string()),
            status: status.to_string(),
            pane_id: Some("w1:p1".to_string()),
            cwd: Some("/x".to_string()),
        }
    }

    /// A settable clock: returns whatever the handle was last set to.
    fn fake_clock() -> (Rc<Cell<u64>>, Box<dyn Fn() -> u64>) {
        let cell = Rc::new(Cell::new(0u64));
        let handle = Rc::clone(&cell);
        (cell, Box::new(move || handle.get()))
    }

    #[test]
    fn measures_time_in_the_current_status() {
        let (clock, now) = fake_clock();
        clock.set(1000);
        let mut t = AttentionTracker::with_clock(now);
        t.observe(&[agent("a", "blocked")]);
        clock.set(6000);
        assert_eq!(t.in_status_ms("a"), 5000);
    }

    #[test]
    fn resets_the_timer_when_the_status_changes() {
        let (clock, now) = fake_clock();
        let mut t = AttentionTracker::with_clock(now);
        t.observe(&[agent("a", "working")]);
        clock.set(10_000);
        t.observe(&[agent("a", "blocked")]); // status changed → since resets to now
        clock.set(12_000);
        assert_eq!(t.in_status_ms("a"), 2000);
    }

    #[test]
    fn keeps_the_timer_running_while_the_status_is_unchanged() {
        let (clock, now) = fake_clock();
        let mut t = AttentionTracker::with_clock(now);
        t.observe(&[agent("a", "blocked")]);
        clock.set(3000);
        t.observe(&[agent("a", "blocked")]); // same status → since unchanged
        clock.set(8000);
        assert_eq!(t.in_status_ms("a"), 8000);
    }

    #[test]
    fn forgets_agents_that_disappear() {
        let (clock, now) = fake_clock();
        let mut t = AttentionTracker::with_clock(now);
        t.observe(&[agent("a", "blocked")]);
        clock.set(5000);
        t.observe(&[]);
        assert_eq!(t.in_status_ms("a"), 0);
    }

    #[test]
    fn returns_zero_for_an_unknown_agent() {
        let (_clock, now) = fake_clock();
        let t = AttentionTracker::with_clock(now);
        assert_eq!(t.in_status_ms("nope"), 0);
    }

    #[test]
    fn a_backwards_clock_does_not_produce_a_huge_age() {
        let (clock, now) = fake_clock();
        clock.set(10_000);
        let mut t = AttentionTracker::with_clock(now);
        t.observe(&[agent("a", "blocked")]);
        clock.set(4_000); // e.g. an NTP step
        assert_eq!(t.in_status_ms("a"), 0);
    }
}
