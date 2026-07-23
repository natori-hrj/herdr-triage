//! Attention triage for herdr: poll `agent.list`, rank by who needs you most, redraw.

mod config;
mod herdr;
mod json;
mod priority;
mod render;
mod tracker;

use std::io::Write;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use crate::herdr::HerdrClient;
use crate::priority::{rank, RankItem};
use crate::render::render_triage;
use crate::tracker::AttentionTracker;

fn main() -> ExitCode {
    let cfg = config::load_config();
    let client = HerdrClient::new();

    if !client.ping() {
        eprintln!("herdr socket not reachable — is a herdr server running?");
        return ExitCode::FAILURE;
    }

    let mut tracker = AttentionTracker::new();
    let interval = Duration::from_millis(cfg.poll_interval_ms);

    loop {
        match client.agent_list() {
            Ok(agents) => {
                tracker.observe(&agents);
                let items: Vec<RankItem> = agents
                    .iter()
                    .map(|a| RankItem::from_agent(a, tracker.in_status_ms(&a.terminal_id)))
                    .collect();
                // Clear the pane and redraw the ranked list each tick (a live triage view).
                println!("\x1b[2J\x1b[H{}", render_triage(&rank(items, &cfg)));
                let _ = std::io::stdout().flush();
            }
            // A failed poll leaves the last good list on screen rather than blanking it —
            // a momentary socket hiccup shouldn't wipe the view you were reading.
            Err(e) => eprintln!("triage: {}", e),
        }
        thread::sleep(interval);
    }
}
