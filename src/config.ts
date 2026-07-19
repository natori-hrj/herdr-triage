import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export interface Config {
  /** Base weight per status — higher means "needs you sooner". */
  weights: { blocked: number; done: number; working: number; idle: number; default: number };
  /** Extra priority per second a blocked agent has been waiting. */
  waitBonusPerSec: number;
  /** Cap on the wait bonus so one very old block doesn't dominate forever. */
  maxWaitBonus: number;
  /** How often to refresh, in milliseconds. */
  pollIntervalMs: number;
}

export const DEFAULTS: Config = {
  weights: { blocked: 1000, done: 500, working: 100, idle: 10, default: 50 },
  waitBonusPerSec: 1,
  maxWaitBonus: 600, // 10 minutes' worth
  pollIntervalMs: 1500,
};

export type FileReader = (file: string) => string | null;

const readFromDisk: FileReader = (file) => {
  try {
    return fs.readFileSync(file, "utf8");
  } catch {
    return null;
  }
};

/** Load config from $HERDR_PLUGIN_CONFIG_DIR/config.json, else ~/.config/herdr-triage. */
export function loadConfig(readFile: FileReader = readFromDisk): Config {
  const dir =
    process.env.HERDR_PLUGIN_CONFIG_DIR || path.join(os.homedir(), ".config", "herdr-triage");
  const raw = readFile(path.join(dir, "config.json"));
  const fromFile = raw ? (JSON.parse(raw) as Partial<Config>) : {};
  return {
    ...DEFAULTS,
    ...fromFile,
    weights: { ...DEFAULTS.weights, ...fromFile.weights },
  };
}
