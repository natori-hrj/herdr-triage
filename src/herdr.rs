//! Minimal client for herdr's control socket: one connection per request.
//!
//! The protocol is newline-delimited JSON — write `{"id","method","params"}\n`, read one
//! line back. Triage only ever calls `ping` and `agent.list`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use crate::json::{self, Value};

/// The fields Triage reads from an `agent.list` entry.
///
/// Everything except the two ids is optional: herdr reports `agent`, `cwd` and
/// `terminal_title_stripped` as null for a pane it hasn't identified yet, and a pane in
/// that state still belongs in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub terminal_id: String,
    pub agent: Option<String>,
    pub task: Option<String>,
    pub status: String,
    pub pane_id: Option<String>,
    pub cwd: Option<String>,
}

/// Status reported for a pane whose agent state herdr can't determine.
const UNKNOWN_STATUS: &str = "unknown";

pub fn resolve_socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("HERDR_SOCKET_PATH") {
        return PathBuf::from(p);
    }
    crate::config::home_dir()
        .join(".config")
        .join("herdr")
        .join("herdr.sock")
}

pub struct HerdrClient {
    socket_path: PathBuf,
}

impl HerdrClient {
    pub fn new() -> Self {
        HerdrClient {
            socket_path: resolve_socket_path(),
        }
    }

    /// Send one request and return the response envelope. `params` is always empty for the
    /// two methods Triage calls, so it isn't a parameter.
    fn request(&self, method: &str, timeout: Duration) -> Result<Value, String> {
        let mut sock = UnixStream::connect(&self.socket_path)
            .map_err(|e| format!("connect {}: {}", self.socket_path.display(), e))?;
        sock.set_read_timeout(Some(timeout)).map_err(fmt_io)?;
        sock.set_write_timeout(Some(timeout)).map_err(fmt_io)?;

        let mut req = String::from("{\"id\":");
        json::write_str(&mut req, "1");
        req.push_str(",\"method\":");
        json::write_str(&mut req, method);
        req.push_str(",\"params\":{}}\n");
        sock.write_all(req.as_bytes())
            .map_err(|e| format!("herdr {}: {}", method, e))?;

        let line = read_line(&mut sock).map_err(|e| format!("herdr {}: {}", method, e))?;
        let msg = json::parse(&line)?;
        if let Some(err) = msg.get("error") {
            let code = err.str_field("code").unwrap_or("?");
            let message = err.str_field("message").unwrap_or("(no message)");
            return Err(format!("herdr {}: {} {}", method, code, message));
        }
        if msg.get("result").is_none() {
            return Err(format!("herdr {}: response had no result", method));
        }
        Ok(msg)
    }

    pub fn agent_list(&self) -> Result<Vec<Agent>, String> {
        let msg = self.request("agent.list", Duration::from_secs(4))?;
        Ok(parse_agents(&msg))
    }

    pub fn ping(&self) -> bool {
        self.request("ping", Duration::from_millis(1500)).is_ok()
    }
}

impl Default for HerdrClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Pull the agent list out of an `agent.list` response envelope.
///
/// An entry with no `terminal_id` is skipped rather than given a placeholder: the id is
/// what the tracker times an agent by, so two id-less entries would share one timer.
pub fn parse_agents(msg: &Value) -> Vec<Agent> {
    let Some(items) = msg.path(&["result", "agents"]).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|a| {
            Some(Agent {
                terminal_id: a.str_field("terminal_id")?.to_string(),
                agent: a.str_field("agent").map(str::to_string),
                task: a.str_field("terminal_title_stripped").map(str::to_string),
                status: a
                    .str_field("agent_status")
                    .unwrap_or(UNKNOWN_STATUS)
                    .to_string(),
                pane_id: a.str_field("pane_id").map(str::to_string),
                cwd: a.str_field("cwd").map(str::to_string),
            })
        })
        .collect()
}

/// Read bytes until the first newline. The response is one line, but it arrives in as many
/// chunks as the kernel feels like giving us, so a single `read` isn't enough.
fn read_line(sock: &mut UnixStream) -> std::io::Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = sock.read(&mut chunk)?;
        if n == 0 {
            break; // peer closed; parse whatever we have and let JSON report the truncation
        }
        if let Some(nl) = chunk[..n].iter().position(|&b| b == b'\n') {
            buf.extend_from_slice(&chunk[..nl]);
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn fmt_io(e: std::io::Error) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_LIST: &str = r#"{"id":"1","result":{"agents":[
        {"agent":"claude","agent_status":"blocked","cwd":"/w","pane_id":"w1:pA",
         "terminal_id":"term-1","terminal_title_stripped":"Migrate the auth module"},
        {"agent":null,"agent_status":"unknown","cwd":null,"pane_id":"w1:pB",
         "terminal_id":"term-2","terminal_title_stripped":null}
    ],"type":"agent_list"}}"#;

    /// Wrap a bare `agents` array in the response envelope the client actually receives.
    fn envelope(agents: &str) -> Value {
        json::parse(&format!(r#"{{"id":"1","result":{{"agents":{}}}}}"#, agents)).unwrap()
    }

    #[test]
    fn reads_agents_from_a_real_result() {
        let agents = parse_agents(&json::parse(REAL_LIST).unwrap());
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].terminal_id, "term-1");
        assert_eq!(agents[0].status, "blocked");
        assert_eq!(agents[0].task.as_deref(), Some("Migrate the auth module"));
        assert_eq!(agents[0].pane_id.as_deref(), Some("w1:pA"));
    }

    #[test]
    fn null_fields_survive_as_none() {
        let agents = parse_agents(&json::parse(REAL_LIST).unwrap());
        assert_eq!(agents[1].agent, None);
        assert_eq!(agents[1].task, None);
        assert_eq!(agents[1].cwd, None);
        assert_eq!(agents[1].status, "unknown");
    }

    #[test]
    fn an_entry_without_a_terminal_id_is_skipped() {
        let agents = parse_agents(&envelope(r#"[{"agent_status":"idle","pane_id":"w1:pA"}]"#));
        assert!(agents.is_empty());
    }

    #[test]
    fn a_missing_status_reads_as_unknown() {
        let agents = parse_agents(&envelope(r#"[{"terminal_id":"t"}]"#));
        assert_eq!(agents[0].status, UNKNOWN_STATUS);
    }

    #[test]
    fn an_empty_or_absent_list_is_no_agents() {
        assert!(parse_agents(&envelope("[]")).is_empty());
        assert!(parse_agents(&json::parse(r#"{"id":"1","result":{}}"#).unwrap()).is_empty());
    }

    /// The envelope's own `id` must not be read as an agent field — the reason this goes
    /// through a real parser instead of a substring scan.
    #[test]
    fn the_envelope_id_is_not_mistaken_for_an_agent() {
        let agents = parse_agents(&json::parse(REAL_LIST).unwrap());
        assert_eq!(agents[0].terminal_id, "term-1");
        assert_eq!(agents[1].terminal_id, "term-2");
    }
}
