import { loadConfig } from "./config.js";
import { HerdrClient } from "./herdr.js";
import { rank, RankItem } from "./priority.js";
import { renderTriage } from "./render.js";
import { AttentionTracker } from "./tracker.js";

async function main(): Promise<void> {
  const cfg = loadConfig();
  const herdr = new HerdrClient();

  if (!(await herdr.ping())) {
    console.error("herdr socket not reachable — is a herdr server running?");
    process.exit(1);
  }

  const tracker = new AttentionTracker();

  const tick = async () => {
    const agents = await herdr.agentList();
    tracker.observe(agents);
    const items: RankItem[] = agents.map((a) => ({
      terminalId: a.terminal_id,
      agent: a.agent,
      paneId: a.pane_id,
      task: a.terminal_title_stripped,
      cwd: a.cwd,
      status: a.agent_status,
      inStatusMs: tracker.inStatusMs(a.terminal_id),
    }));
    // Clear the pane and redraw the ranked list each tick (a live triage view).
    process.stdout.write("\x1b[2J\x1b[H");
    console.log(renderTriage(rank(items, cfg)));
  };

  await tick();
  const timer = setInterval(() => tick().catch((e) => console.error((e as Error).message)), cfg.pollIntervalMs);

  const shutdown = () => {
    clearInterval(timer);
    process.exit(0);
  };
  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
