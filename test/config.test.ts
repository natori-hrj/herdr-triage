import { describe, expect, it } from "vitest";
import { DEFAULTS, loadConfig } from "../src/config.js";

describe("loadConfig", () => {
  it("returns defaults when no config file exists", () => {
    expect(loadConfig(() => null)).toEqual(DEFAULTS);
  });

  it("deep-merges weights over the defaults", () => {
    const c = loadConfig(() => JSON.stringify({ weights: { blocked: 2000 } }));
    expect(c.weights.blocked).toBe(2000);
    expect(c.weights.done).toBe(DEFAULTS.weights.done); // other weights preserved
  });
});
