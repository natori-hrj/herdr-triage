import { describe, expect, it } from "vitest";
import { Agent } from "../src/herdr.js";
import { AttentionTracker } from "../src/tracker.js";

const agent = (id: string, status: string): Agent => ({
  terminal_id: id,
  agent: "claude",
  terminal_title_stripped: "task",
  agent_status: status,
  pane_id: "w1:p1",
  cwd: "/x",
});

describe("AttentionTracker", () => {
  it("measures time in the current status with an injected clock", () => {
    let now = 1000;
    const t = new AttentionTracker(() => now);
    t.observe([agent("a", "blocked")]);
    now = 6000;
    expect(t.inStatusMs("a")).toBe(5000);
  });

  it("resets the timer when the status changes", () => {
    let now = 0;
    const t = new AttentionTracker(() => now);
    t.observe([agent("a", "working")]);
    now = 10_000;
    t.observe([agent("a", "blocked")]); // status changed → since resets to now
    now = 12_000;
    expect(t.inStatusMs("a")).toBe(2000);
  });

  it("keeps the timer running while the status is unchanged", () => {
    let now = 0;
    const t = new AttentionTracker(() => now);
    t.observe([agent("a", "blocked")]);
    now = 3000;
    t.observe([agent("a", "blocked")]); // same status → since unchanged
    now = 8000;
    expect(t.inStatusMs("a")).toBe(8000);
  });

  it("forgets agents that disappear", () => {
    let now = 0;
    const t = new AttentionTracker(() => now);
    t.observe([agent("a", "blocked")]);
    now = 5000;
    t.observe([]); // 'a' gone
    expect(t.inStatusMs("a")).toBe(0);
  });

  it("returns 0 for an unknown agent", () => {
    const t = new AttentionTracker(() => 0);
    expect(t.inStatusMs("nope")).toBe(0);
  });
});
