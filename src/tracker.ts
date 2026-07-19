import { Agent } from "./herdr.js";

interface Entry {
  status: string;
  since: number;
}

/**
 * Tracks how long each agent has been in its current status by diffing
 * successive `agent.list` snapshots. herdr doesn't timestamp status changes, so
 * we time them ourselves. The clock is injected so behavior is deterministic
 * under test.
 */
export class AttentionTracker {
  private entries = new Map<string, Entry>();

  constructor(private readonly now: () => number = () => Date.now()) {}

  /** Record a fresh snapshot: reset the timer for any agent whose status changed. */
  observe(agents: Agent[]): void {
    const t = this.now();
    const seen = new Set<string>();
    for (const a of agents) {
      seen.add(a.terminal_id);
      const prev = this.entries.get(a.terminal_id);
      if (!prev || prev.status !== a.agent_status) {
        this.entries.set(a.terminal_id, { status: a.agent_status, since: t });
      }
    }
    for (const id of this.entries.keys()) {
      if (!seen.has(id)) this.entries.delete(id);
    }
  }

  /** Milliseconds the agent has been in its current status (0 if unknown). */
  inStatusMs(terminalId: string): number {
    const e = this.entries.get(terminalId);
    return e ? Math.max(0, this.now() - e.since) : 0;
  }
}
