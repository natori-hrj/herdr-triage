import { describe, expect, it } from "vitest";
import { DEFAULTS } from "../src/config.js";
import { rank, RankItem, score } from "../src/priority.js";

const item = (over: Partial<RankItem>): RankItem => ({
  terminalId: "t",
  agent: "claude",
  paneId: "w1:p1",
  task: "task",
  cwd: "/x",
  status: "idle",
  inStatusMs: 0,
  ...over,
});

describe("score", () => {
  it("orders blocked > done > working > idle at equal time", () => {
    const b = score("blocked", 0, DEFAULTS);
    const d = score("done", 0, DEFAULTS);
    const w = score("working", 0, DEFAULTS);
    const i = score("idle", 0, DEFAULTS);
    expect(b).toBeGreaterThan(d);
    expect(d).toBeGreaterThan(w);
    expect(w).toBeGreaterThan(i);
  });

  it("raises a blocked agent's score the longer it waits", () => {
    expect(score("blocked", 60_000, DEFAULTS)).toBeGreaterThan(score("blocked", 0, DEFAULTS));
  });

  it("caps the wait bonus", () => {
    const huge = score("blocked", 10 ** 12, DEFAULTS);
    expect(huge).toBe(DEFAULTS.weights.blocked + DEFAULTS.maxWaitBonus);
  });

  it("does not apply a time bonus to working or idle", () => {
    expect(score("working", 10 ** 9, DEFAULTS)).toBe(DEFAULTS.weights.working);
    expect(score("idle", 10 ** 9, DEFAULTS)).toBe(DEFAULTS.weights.idle);
  });

  it("raises a done agent's score the longer it sits, capped", () => {
    expect(score("done", 60_000, DEFAULTS)).toBeGreaterThan(score("done", 0, DEFAULTS));
    expect(score("done", 10 ** 12, DEFAULTS)).toBe(DEFAULTS.weights.done + DEFAULTS.maxDoneBonus);
  });

  it("keeps a maxed-out done agent below a just-blocked agent", () => {
    expect(score("done", 10 ** 12, DEFAULTS)).toBeLessThan(score("blocked", 0, DEFAULTS));
  });

  it("uses the default weight for unknown statuses", () => {
    expect(score("frobnicating", 0, DEFAULTS)).toBe(DEFAULTS.weights.default);
  });
});

describe("rank", () => {
  it("puts the longest-blocked agent first", () => {
    const ranked = rank(
      [
        item({ terminalId: "a", status: "blocked", inStatusMs: 5_000 }),
        item({ terminalId: "b", status: "blocked", inStatusMs: 120_000 }),
        item({ terminalId: "c", status: "done" }),
        item({ terminalId: "d", status: "idle" }),
      ],
      DEFAULTS,
    );
    expect(ranked.map((r) => r.terminalId)).toEqual(["b", "a", "c", "d"]);
  });

  it("breaks score ties by longer time-in-status", () => {
    const ranked = rank(
      [
        item({ terminalId: "x", status: "working", inStatusMs: 1_000 }),
        item({ terminalId: "y", status: "working", inStatusMs: 9_000 }),
      ],
      DEFAULTS,
    );
    expect(ranked[0].terminalId).toBe("y");
  });
});
