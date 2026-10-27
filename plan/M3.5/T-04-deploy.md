---
id: T-04
title: "fa-server init, compose, Tailscale sidecar, release image"
milestone: M3.5
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["bins/fa-server/src/init.rs", "bins/fa-server/src/cli.rs", "bins/fa-server/src/main.rs", "deploy/compose.yaml", "deploy/compose.build.yaml", "deploy/tailscale/**", "deploy/.env.example", ".github/workflows/release-image.yml"]
decisions: [D-065, D-069, D-072, D-074, D-076, D-077]
---

# T-04: fa-server init, compose, Tailscale sidecar, release image

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md`: Workstream A and *Contracts fixed up front* → Compose, Server
- `arch/Platform.md` §3, §8, §9
- `deploy/compose.example.yaml` (today's file: carry over postgres, ntfy, backup, the gpu/ai-cpu profiles), `Dockerfile` (uid 10001), `bins/fa-server/src/{main,cli}.rs`
- Tailscale container docs: https://tailscale.com/kb/1282/docker (verify `TS_AUTHKEY`, `TS_HOSTNAME`, `TS_STATE_DIR`, `TS_USERSPACE`, `TS_SERVE_CONFIG` and the `${TS_CERT_DOMAIN}` placeholder there before using them)

## You own
- `bins/fa-server/src/init.rs`
- `bins/fa-server/src/cli.rs`
- `bins/fa-server/src/main.rs`
- `deploy/compose.yaml`
- `deploy/compose.build.yaml`
- `deploy/tailscale/**`
- `deploy/.env.example`
- `.github/workflows/release-image.yml`

## Work
1. **`fa-server init [--secrets-dir <dir>]`** (default `FA_SECRETS_DIR` or `/run/fa-secrets`), dispatched in `main.rs` **before** `ProcessConfig::load`, so it needs no database URL. Creates `master_key` (32 bytes from `rand`'s OS RNG, 0400, chown 10001:10001) and `db_password` (48 lower-case hex, 0444) if absent; never overwrites; prints only which files it created. Put the logic in a pure function `create_secrets(dir: &Path, owner: Option<(u32, u32)>) -> std::io::Result<Vec<&'static str>>` (the names it created) with unit tests on a temp dir (skip the chown assertion when not root). **`fa-server` may not compile during W1**, because T-02 is changing `fa-domain` at the same time: write the code and tests, run them if the crate compiles, and otherwise report that. The main session runs them after W4.
2. **`deploy/compose.yaml`** per the contract: `init` (same image, `user: "0:0"`, `command: ["init"]`), `postgres` (`POSTGRES_USER=fa`, `POSTGRES_DB=fa`, `POSTGRES_PASSWORD_FILE`), `app` (`serve all`, healthcheck on `/readyz`, publishes `${FA_HTTP_HOST_IP:-0.0.0.0}:${FA_HTTP_PORT:-8080}:8080`), `ntfy` (pinned as today, published on `${FA_NTFY_PORT:-8090}`), `backup` (volume `fa-backups`, or `BACKUP_DIR`), profile `tailscale` (pin a real version tag of `tailscale/tailscale`, verified with `docker manifest inspect`; userspace; state volume `ts-state`; fixed IP `172.30.89.10`; `TS_HOSTNAME` default `family`), profiles `gpu`/`ai-cpu` unchanged. Image `ghcr.io/jonganz/fa-server:${FA_VERSION:-latest}`. **No variable is required**: `docker compose config` must succeed with no `.env`.
3. **Serve config:** HTTPS 443 → `http://app:8080`, 8443 → `http://ntfy:80`, using `${TS_CERT_DOMAIN}`. Prefer a top-level compose `configs:` entry with inline `content:`, so the install stays **one downloadable file**. Check the minimum Compose version that supports it in the Compose docs (the local one is v5.4.0) and state it in `.env.example`. Fall back to `deploy/tailscale/serve.json` only if inline content can't keep `${TS_CERT_DOMAIN}` literal (`$$` escaping).
4. **`deploy/compose.build.yaml`**: adds `build: {context: .., dockerfile: Dockerfile}` to `init` and `app`.
5. **`deploy/.env.example`**: every variable optional, grouped (ports, version, backups, Tailscale, advanced overrides), each with one line of explanation. No secrets.
6. **`.github/workflows/release-image.yml`**: on `v*` tags, buildx `linux/amd64`, push `ghcr.io/jonganz/fa-server:<tag>` and `latest` with `GITHUB_TOKEN` (`packages: write`). Use only official actions (`docker/setup-buildx-action`, `docker/login-action`, `docker/build-push-action`, `docker/metadata-action`) at versions you verify exist.
7. Leave `deploy/compose.example.yaml` and `compose.dev.yaml` alone (T-10 retires the former).

## Acceptance
- [ ] the init tests pass, or the report says the crate did not compile because of other tasks' changes (then the main session runs them after W4)
- [ ] `docker compose -f deploy/compose.yaml config` and `docker compose -f deploy/compose.yaml --profile tailscale config` succeed with no `.env`
- [ ] `init` in a real container (fresh volume, modes, owners, idempotent) is verified by T-10, not here
- [ ] the workflow file is valid YAML (`python3 -c 'import yaml,sys; yaml.safe_load(open(sys.argv[1]))' <file>`)
