---
id: T-17
title: "Docs: README AI setup, roadmap, deploy notes"
milestone: M4
wave: 4
depends_on: [T-08]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["README.md", "deploy/**", "docs/**"]
decisions: [D-093, D-094, D-095, D-096, D-097]
---

# T-17: Docs: README AI setup, roadmap, deploy notes

You are a sub-agent implementing one task of milestone M4 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M4 adds an LLM, reached over a configurable OpenAI-compatible endpoint (default Ollama on a GPU PC, D-093), and revamps the app UI (D-095, D-096). You bring the user-facing docs in line. **The repo is public (D-035):** no household specifics, and only example addresses (`192.168.1.20`, `gpu-pc.example.ts.net`).

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`.
- `plan/M4.md`: *Why*, *Done means*, *API (T-08)* and *Eval (T-09)*.
- `arch/AI.md` §2, §11 and §12; `arch/Platform.md` §5 (the LLM endpoint row) and §8.
- Decisions D-093, D-094, D-097.
- `research/R-004-openai-compatible-llm-api.md` (Ollama port, `OLLAMA_HOST`, `/v1`, the model tag `qwen3:8b`).
- `README.md` in full (the Roadmap table, the layout section, "no AI model runs yet" wording); `deploy/README.md` and `deploy/compose.yaml` (the `gpu` and `ai-cpu` profiles).
- The reports of T-08 and T-09 (exact routes, the Settings → AI screen name, the `eval` synopsis).

## You own
`README.md`, `deploy/**` and `docs/**` (if present). Nothing else.

## Work
1. **README, new section "Local AI (optional)":**
   - What it does in M4: it classifies mail the rules can't settle and extracts events, bills and tasks for the approval inbox. Nothing is applied without approval.
   - Any OpenAI-compatible server works.
   - **Setup on the GPU PC:** install Ollama (official installer: link to ollama.com/download, and don't paste a `curl | sh`). Set `OLLAMA_HOST` to the PC's **LAN or tailnet IP** and port 11434, with the systemd override shown (`systemctl edit ollama` → `Environment="OLLAMA_HOST=192.168.1.20:11434"`). Then run `ollama pull qwen3:8b`.
   - **Security warning:** Ollama has no authentication. Never bind it to a public address or forward its port.
   - In the app: Settings → AI → base URL `http://192.168.1.20:11434/v1`, the model, "Test connection", then enable it, and the per-decision-point toggles.
   - What happens while the PC sleeps: jobs wait and retry for about a day, and rules keep working.
   - The `eval` command (from T-09's report) and that it reads only the household's labelled sample.
2. **README** wording:
   - Replace "no AI model runs yet (M4 adds those…)" with the M4 state.
   - Layout section: `crates/fa-llm`, `fa-recipe`, `fa-eval`; `fa-gpu-agent` is described as an M4.1 optional helper.
   - Roadmap rows, per D-094 and D-097:
     - M4: "UI revamp (component library, navigation, one apply-changes model) and the LLM extraction slice (OpenAI-compatible endpoint, `llm` stage for email, recipe extraction for M5, basic eval)";
     - M4.1: "System One sidecar, kNN, backend registry, optional GPU agent with WoL, full eval";
     - leave the status column for the main session to fill in at close.
3. **`deploy/`:**
   - In `deploy/README.md`, say that the `gpu` profile's `ollama` service is one option and that a natively installed Ollama is the default; the `ai-cpu` profile is M4.1.
   - In `compose.yaml`, edit only comments: mark `ai-cpu` as M4.1. If the `ollama` service exists, check its image tag with `docker manifest inspect ollama/ollama:<tag>` before naming it. If it doesn't exist, don't add it; report instead.
4. **Verify every command you write:**
   - `docker manifest inspect` for any image;
   - `cargo run -p fa-server -- eval --help` for the eval synopsis;
   - check that the ollama.com download URL and the `qwen3:8b` tag exist on ollama.com/library (WebFetch).
   Commands you could not run on this machine (installing Ollama) are listed in the report as unverified.

## Acceptance
- [ ] `python3 tools/fa.py check` passes, and `scripts/ci-local.sh --no-web` passes its docs and hygiene steps (report the result).
- [ ] No household specifics, real IPs or names: `grep -nE "@(gmail|outlook)\.com" README.md deploy/` is empty, and only the example addresses appear.
- [ ] The report lists:
  - sections changed;
  - each command and how it was verified;
  - unverified items;
  - "list what outside your paths needs changing" (for example, an arch doc that disagrees).
