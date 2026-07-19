# Herdr Triage

Attention **triage** for [herdr](https://herdr.dev). When you're running many
agents, the bottleneck is *you* — and it's easy to miss the one that's been
blocked for ten minutes. Herdr Triage ranks every agent by who needs you most
and shows them in order, so you always know who to deal with first.

```
Attention triage — 4 agent(s)
🔴   9m  Migrate the auth module           w1:pA
🔴   1m  Update the changelog              w1:pB
✅   —   Add a dark-mode toggle            w1:pC
⚙️   —   Investigate the flaky test        w1:pD
```

## How it ranks

- **Blocked** agents rank highest — and rise further the longer they've been
  waiting (herdr doesn't timestamp status changes, so Triage times them itself
  by polling).
- **Done** agents come next (they need a review or a next task).
- **Working** and **idle** sit at the bottom.

All weights and the wait bonus are configurable.

Inspired by [herdr#318](https://github.com/ogulcancelik/herdr/issues/318)
("sort agents panel by attention priority").

## Security

Triage is **read-only**: it calls `agent.list` and renders a list. It runs no
commands and mutates nothing — no repos, no panes, no agents.

## Install (as a herdr plugin)

```bash
herdr plugin install <owner>/herdr-triage
herdr plugin pane open triage/list
```

## Develop

```bash
npm install
npm test          # unit tests (vitest), deterministic via an injected clock
npm run typecheck
npm run build
```

## Status

Foundation. Priority scoring, time-in-status tracking (with an injected clock for
deterministic tests), rendering, and config are implemented and unit-tested. It
renders a live, self-refreshing list in its pane.

## License

Apache-2.0
