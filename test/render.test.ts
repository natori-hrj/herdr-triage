import { describe, expect, it } from "vitest";
import { Ranked } from "../src/priority.js";
import { formatDuration, renderTriage } from "../src/render.js";

describe("formatDuration", () => {
  it("formats seconds", () => expect(formatDuration(42_000)).toBe("42s"));
  it("formats minutes", () => expect(formatDuration(5 * 60_000)).toBe("5m"));
  it("formats hours and minutes", () => expect(formatDuration(63 * 60_000)).toBe("1h3m"));
});

const r = (over: Partial<Ranked>): Ranked => ({
  terminalId: "t",
  agent: "claude",
  paneId: "w1:p1",
  task: "Fix login",
  cwd: "/x",
  status: "blocked",
  inStatusMs: 300_000,
  score: 1300,
  ...over,
});

describe("renderTriage", () => {
  it("says so when there are no agents", () => {
    expect(renderTriage([])).toBe("No agents.");
  });

  it("shows a wait time for blocked agents and a dash otherwise", () => {
    const out = renderTriage([r({ status: "blocked", inStatusMs: 300_000 }), r({ status: "done", paneId: "w1:p2" })]);
    expect(out).toContain("🔴");
    expect(out).toContain("5m");
    expect(out).toContain("✅");
    expect(out).toContain("w1:p2");
  });

  it("includes a header with the agent count", () => {
    expect(renderTriage([r({}), r({ paneId: "w1:p2" })])).toContain("2 agent(s)");
  });
});
