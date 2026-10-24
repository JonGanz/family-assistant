---
id: D-020
title: GPU node (Linux): Wake-on-LAN, auto-wake by schedule/queue depth, yields to gaming via agent
status: accepted
date: 2026-10-23
source_questions: [Q-019]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-020: GPU node (Linux): Wake-on-LAN, auto-wake by schedule/queue depth, yields to gaming via agent

## Context
The GPU node runs Linux. The user picked option A: auto-wake, and yield while a game is running.

## Decision
- **Wake:** the core node sends a WoL magic packet. This needs both machines on the same LAN and WoL enabled in BIOS and on the NIC. Triggers:
  - a schedule (default 03:00 daily);
  - queue depth ≥ N;
  - oldest job age ≥ T.
- **`fa-gpu-agent`** is a small Rust systemd service on the GPU node, from the same workspace. It:
  - sends a heartbeat to the core with its state (`available` or `busy_gaming`);
  - detects gaming from configurable signals: a Steam game process, Feral `gamemoded` being active, or GPU utilisation from `nvidia-smi` while Ollama is idle;
  - unloads Ollama models when it's busy, using `keep_alive: 0`.
- The gateway drains the queue only while the state is `available`.
- **Auto-suspend** (configurable, default on): if the system woke the node, the queue is empty, and no user session is active for 15 minutes, the agent suspends it.

## Consequences
- The design needs an agent-to-core auth token, and nothing is reachable except over the LAN or tailnet.

## Alternatives considered
Never auto-wake. Not chosen.
