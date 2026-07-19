import { Config } from "./config.js";
import { AgentStatus } from "./herdr.js";

export interface RankItem {
  terminalId: string;
  agent: string;
  paneId: string;
  task: string;
  cwd: string;
  status: AgentStatus;
  /** How long the agent has been in its current status (ms). */
  inStatusMs: number;
}

export interface Ranked extends RankItem {
  score: number;
}

/**
 * Priority score for an agent. Blocked agents rank highest and rise further the
 * longer they've been waiting (capped); done agents (need review / a next task)
 * come next; working and idle are low. Pure — trivially unit-testable.
 */
export function score(status: AgentStatus, inStatusMs: number, cfg: Config): number {
  const base =
    status === "blocked"
      ? cfg.weights.blocked
      : status === "done"
        ? cfg.weights.done
        : status === "working"
          ? cfg.weights.working
          : status === "idle"
            ? cfg.weights.idle
            : cfg.weights.default;

  if (status === "blocked") {
    return base + Math.min(cfg.maxWaitBonus, (inStatusMs / 1000) * cfg.waitBonusPerSec);
  }
  if (status === "done") {
    // A done agent that's sat unreviewed nudges up so it isn't buried under
    // fresh done agents — capped so it never overtakes a blocked agent.
    return base + Math.min(cfg.maxDoneBonus, (inStatusMs / 1000) * cfg.doneBonusPerSec);
  }
  return base;
}

/** Rank agents by attention priority, highest first. Ties break on longer wait. */
export function rank(items: RankItem[], cfg: Config): Ranked[] {
  return items
    .map((it) => ({ ...it, score: score(it.status, it.inStatusMs, cfg) }))
    .sort((a, b) => b.score - a.score || b.inStatusMs - a.inStatusMs);
}
