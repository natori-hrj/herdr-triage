import { Ranked } from "./priority.js";

const GLYPH: Record<string, string> = { blocked: "🔴", done: "✅", working: "⚙️", idle: "💤" };

/** Human-friendly duration: "42s", "5m", "1h3m". Pure. */
export function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  return `${h}h${m % 60}m`;
}

function trunc(s: string, n: number): string {
  return s.length > n ? s.slice(0, n - 1) + "…" : s;
}

/** Render the ranked agents as a compact, glanceable triage list. Pure. */
export function renderTriage(ranked: Ranked[]): string {
  if (ranked.length === 0) return "No agents.";
  const lines = [`Attention triage — ${ranked.length} agent(s)`];
  for (const r of ranked) {
    const glyph = GLYPH[r.status] ?? "·";
    const wait = r.status === "blocked" ? formatDuration(r.inStatusMs).padStart(5) : "  —  ";
    lines.push(`${glyph} ${wait}  ${trunc(r.task || r.paneId, 32).padEnd(32)}  ${r.paneId}`);
  }
  return lines.join("\n");
}
