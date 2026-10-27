---
id: T-09
title: "Docs: deploy guide, README, runbooks"
milestone: M3.5
wave: 5
depends_on: [T-02, T-03, T-04, T-05, T-06, T-07, T-08]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["README.md", "deploy/README.md", "deploy/runbooks/**", "config/examples/**"]
decisions: [D-065, D-072, D-073, D-074, D-075, D-076, D-077]
---

# T-09: Docs: deploy guide, README, runbooks

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md` (all), `arch/Platform.md` v4 §3, §6, §8, §9
- `deploy/compose.yaml`, `deploy/.env.example`, the reports of T-02 to T-08 (added below)
- CLAUDE.md, *README*: what must stay current

## You own
- `README.md`
- `deploy/README.md`
- `deploy/runbooks/**`
- `config/examples/**`

## Work
1. **`deploy/README.md` rewrite:**
   - **Install in three steps:** get Docker; download `compose.yaml` (raw GitHub URL of the release tag) or clone; `docker compose up -d`, then open `http://<server>:8080` and **finish setup right away** (D-073).
   - **Remote access:** the Tailscale profile (auth key, `TS_HOSTNAME`, phones on the tailnet, HTTPS address, ntfy on `:8443`).
   - **Advanced:** Tailscale on the host; your own reverse proxy (add it to trusted proxies in Settings, System); building locally (`compose.build.yaml`, also for arm64, D-077); the D-064 dashboard-only listener.
   - **Backups and the secrets volume:** what to copy off the machine, and the master-key backup from the wizard.
   - **Updates.**
   - **Troubleshooting.**
   - Calendars, mail and phones now point to the UI. Drop every `.env`-required step, the household TOML step and the `fa-server user add`/`pin set` steps (mention them under *Recovery* only).
2. **`README.md`:**
   - Deploying section;
   - Configuration reference: new env vars, `FA_DEFAULT_*`, which settings live in Settings, System, and that env locks them;
   - `fa-server` commands (`init`, `config import --dry-run`);
   - ports;
   - repository layout;
   - the M3.5 roadmap row.
   *Run it locally* keeps working (the dev flow may still use `config import`).
3. **Runbooks:**
   - `install.md`: point to the guide, or fold into it;
   - `restore.md`: the secrets volume, putting the master key back from its base64 backup;
   - `rotate-key.md`: new paths.
4. `config/examples/household.example.toml`: the header comment says it is optional and imported from Settings, System.
5. **Verify by running.** Bring up the guide's install with `-f deploy/compose.yaml -f deploy/compose.build.yaml` under project name `fa-docs`, on ports 18280 (app) and 18290 (ntfy) via `FA_HTTP_PORT`/`FA_NTFY_PORT`. Complete setup with `curl` against the setup API, follow the backup and restore steps, then `docker compose -p fa-docs down -v`. Fix the docs for anything that didn't work as written.

## Acceptance
- [ ] every command in `deploy/README.md` and the runbooks was run, or is marked as needing real hardware (Tailscale auth key, phones)
- [ ] no reference to `compose.example.yaml`, a required `.env` value, `FA_SETUP_CODE`, `lan_mode`, or arm64 images remains
- [ ] `python3 tools/fa.py check` clean; `docker ps -a --filter name=fa-docs` and `docker volume ls -f dangling=true` show nothing left from you
