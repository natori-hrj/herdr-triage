import net from "node:net";
import os from "node:os";
import path from "node:path";

export type AgentStatus = "idle" | "working" | "blocked" | "done" | (string & {});

/** Fields we use from herdr's `agent.list` (verified vs herdr 0.7.4). */
export interface Agent {
  terminal_id: string;
  agent: string;
  terminal_title_stripped: string;
  agent_status: AgentStatus;
  pane_id: string;
  cwd: string;
}

export function resolveSocketPath(): string {
  return (
    process.env.HERDR_SOCKET_PATH || path.join(os.homedir(), ".config", "herdr", "herdr.sock")
  );
}

/** Minimal herdr control-socket client (one connection per request). */
export class HerdrClient {
  constructor(private readonly socketPath: string = resolveSocketPath()) {}

  request<T = unknown>(method: string, params: Record<string, unknown> = {}, timeoutMs = 4000): Promise<T> {
    return new Promise((resolve, reject) => {
      const sock = net.createConnection(this.socketPath);
      let buf = "";
      const done = (fn: () => void) => {
        clearTimeout(timer);
        sock.destroy();
        fn();
      };
      const timer = setTimeout(() => done(() => reject(new Error(`herdr ${method} timed out`))), timeoutMs);
      sock.on("connect", () => sock.write(JSON.stringify({ id: "1", method, params }) + "\n"));
      sock.on("data", (chunk) => {
        buf += chunk;
        const nl = buf.indexOf("\n");
        if (nl < 0) return;
        try {
          const msg = JSON.parse(buf.slice(0, nl));
          if (msg.error) done(() => reject(new Error(`herdr ${method}: ${msg.error.code} ${msg.error.message}`)));
          else done(() => resolve(msg.result as T));
        } catch (e) {
          done(() => reject(e as Error));
        }
      });
      sock.on("error", (e) => done(() => reject(e)));
    });
  }

  async agentList(): Promise<Agent[]> {
    const res = await this.request<{ agents: Agent[] }>("agent.list");
    return res.agents ?? [];
  }

  async ping(): Promise<boolean> {
    try {
      await this.request("ping", {}, 1500);
      return true;
    } catch {
      return false;
    }
  }
}
