---
title: Platform
scope: code layout, runtime, security, deployment, backups, dev workflow
status: draft
updated: 2026-10-26
drawio: Platform.drawio
drawio_src: src/Platform.py
depends_on: [D-002, D-003, D-004, D-005, D-006, D-010, D-018, D-019, D-020, D-023, D-025, D-026, D-031, D-032, D-033, D-034, D-035, D-064, D-065, D-069, D-072, D-073, D-074, D-075, D-076, D-077]
milestone: M0
---

# Platform (DRAFT v4)

The platform that M0 builds and every later milestone depends on. **v4 is the M3.5 Onboarding re-derivation** (D-065, D-069, D-072 to D-077): one compose file with one app container, plain HTTP by default with an optional Tailscale sidecar for HTTPS and remote access, generated secrets, runtime settings in the UI, and a first-run wizard instead of a TOML file and admin commands.

## 1. Repository & workspace layout (page "Packages")
```
/                      this repo: design + code (D-032). Public on GitHub, AGPL-3.0-or-later (D-035)
├─ LICENSE             AGPL-3.0 text
├─ arch/ design/ decisions/ log/ research/ tools/   design system (unchanged)
├─ .github/workflows/  ci.yml, release-image.yml
├─ Cargo.toml          workspace
├─ crates/
│  ├─ fa-domain        entities, IDs, enums, DTOs (ts-rs export), policy types
│  ├─ fa-store         sqlx repos, migrations/, RLS session setup, Secret crypto
│  ├─ fa-config        process config (figment) + household TOML schema, import/export
│  ├─ fa-auth          passwords, sessions, devices, PIN step-up, RBAC policy
│  ├─ fa-jobs          Postgres job queue + scheduler
│  ├─ fa-api           axum routers, OpenAPI (utoipa), SSE hub
│  ├─ fa-notify  fa-connectors  fa-rules  fa-pipeline  fa-decide  fa-llm
│  ├─ fa-backends  fa-sysprobe  fa-eval  fa-calendar  fa-tasks  fa-weather  fa-meals  fa-gpu-node
│  └─ (M0 builds only: domain, store, config, auth, jobs, api, plus the bin crates)
├─ bins/
│  ├─ fa-server        CLI: init · serve {all|api|worker} · migrate · config {import|export} · user {add|reset} · device {pair|revoke} · eval (recovery and scripting; setup is the wizard)
│  └─ fa-gpu-agent     runs on GPU nodes (M4)
├─ apps/web            Vue 3 + TS + Vite + Pinia + vue-router + vite-plugin-pwa (pnpm)
│  └─ src/gen/         generated TS types (ts-rs) and API client (from OpenAPI)
├─ Dockerfile          cargo-chef + node build stages
├─ deploy/
│  ├─ compose.yaml       all services, with profiles: default | tailscale | ai-cpu | gpu (D-033, D-065); the sidecar's serve config is inline (`configs:`, Compose ≥ 2.23.1)
│  ├─ compose.build.yaml override: build the image from this checkout (developers, non-amd64 hosts)
│  ├─ .env.example       optional overrides only; nothing is required (README.md: install guide)
│  ├─ systemd/fa-gpu-agent.service
│  └─ runbooks/           install.md, restore.md, rotate-key.md
├─ config/examples/    household.example.toml (fictional family, D-004)
└─ justfile            optional dev shortcuts (thin wrappers; CI and the Docker build never depend on `just`)
```
**Dependency rule:** `fa-domain` depends on nothing internal. Feature crates depend on `fa-domain`, `fa-store`, and `fa-jobs`, never on each other's internals; they talk through traits defined in `fa-domain`. Only `fa-api` and the bins wire everything together.

## 2. Key libraries
| Concern | Choice |
|---|---|
| Async / HTTP | tokio, axum, tower-http (compression, trace, static files, timeouts) |
| DB | sqlx (postgres, uuid, chrono, json, migrate), with compile-time checked queries (offline `.sqlx` data kept in git) |
| Time | chrono + chrono-tz, rrule |
| Serialisation | serde, serde_json, toml |
| Config | figment (env + file) for process config |
| Auth / crypto | argon2, tower-sessions with a Postgres store, rand, aes-gcm, sha2 |
| API docs / types | utoipa (+ Swagger UI in dev), ts-rs |
| Observability | tracing, tracing-subscriber (JSON in prod) |
| Errors | thiserror in libraries, anyhow in bins. API errors use `application/problem+json` |
| Tests | cargo-nextest, sqlx::test (a throwaway DB per test), insta snapshots. Web uses vitest and Playwright |

## 3. Runtime processes
- One image, `fa-server`, runs as **one `app` container** with `serve all`: HTTP, SSE and the web bundle, plus the scheduler and jobs (the worker role), in one process (D-076). Household scale needs no split; `serve api` and `serve worker` remain in the binary for tests and development but are not part of the deployment.
- **`init` (one-shot):** `fa-server init` runs before Postgres and `app`. It creates `master_key` (32 random bytes) and `db_password` in the `fa-secrets` volume if they are absent, with mode 0400 for the app user, and exits 0. Files the hoster provides are kept (D-074).
- **Startup:** read the DB password from `FA_DATABASE_PASSWORD_FILE` when set, `migrate` under a Postgres advisory lock, load runtime settings (§6), then (M4) `fa-sysprobe`.
- **Setup state:** while no household exists, every `/api/v1/*` call except `/setup*`, `/healthz` and `/readyz` returns `409 setup_required`, and the web app routes to `/setup` (Frontend.md). Setup closes for good once the first adult account exists (D-073).
- **Health:**
  - `/healthz` reports the process is alive.
  - `/readyz` reports the DB is reachable and migrations are current.
  - A **System status** page in the UI shows nodes, backends, sources, the last backup, queue depth, and the connection type (HTTP or HTTPS) as a plain fact.

## 4. API conventions
- REST + JSON under `/api/v1`. OpenAPI is generated, and the TS client is generated from it.
- **SSE** at `/api/v1/stream` pushes typed change events (`task.updated`, `event.proposed`, `weather.updated`, ...). The dashboard and PWA use it to stay live, sending `Last-Event-ID` when they reconnect.
- **Cross-process events (from M1):** writers call `fa_store::ChangeBus::publish` inside their transaction (`pg_notify('fa_change', …)`), and the api's `PgListener` feeds the in-process `Hub`. This way worker-side changes reach SSE clients, and `serve all` uses the same path. See [Feeds.md](Feeds.md) §6.
- **API types:** DTOs derive `utoipa::ToSchema` (from M1), so `openapi.json` is exact and the web client's types are generated from it only.
- Pagination is keyset (cursor), using UUIDv7 ordering. Mutable resources carry a `version` and require `If-Match`, so a conflict returns 409.

## 5. Security (page "Security")
| Area | Design |
|---|---|
| Network | **Default: plain HTTP on the home network.** `app` publishes `${FA_HTTP_HOST_IP:-0.0.0.0}:${FA_HTTP_PORT:-8080}`, and the **entire app** is served there: login, settings, PIN elevation (D-072). Skipping HTTPS is the hoster's choice; the app shows the connection type in Settings, System and does not nag. **Never exposed to the internet** (no port forwarding); the guide says so |
| HTTPS and remote access | **Optional Tailscale sidecar** (compose profile `tailscale`, D-065). `tailscale serve` inside the sidecar terminates HTTPS on `https://<TS_HOSTNAME>.<tailnet>.ts.net` with a real certificate: 443 → `http://app:8080`, 8443 → `http://ntfy:80`. Userspace networking, so no `NET_ADMIN` or `/dev/net/tun`. Any other reverse proxy the hoster runs works the same way. Tailscale on the host (the v3 design) stays as a documented advanced alternative |
| Insecure context | Over HTTP the browser gives no service worker (no PWA install, no offline shell), no `crypto.subtle`, no `crypto.randomUUID`, no async clipboard. The web app must work without them (Frontend.md) |
| Home-network dashboard | Unchanged opt-in (D-064): a second listener (`:8081` in the container) when `FA_LAN_DASHBOARD_URL` (or its Settings, System field) is set, published on `FA_LAN_DASHBOARD_HOST_IP` (default `127.0.0.1`, unreachable). It refuses clients outside `FA_LAN_ALLOWED_CIDRS` (peer address only), routes only the dashboard's own calls, drops cookies and accepts only device tokens, and refuses PIN elevation unless `FA_LAN_ALLOW_PIN=true`. With D-072 it is a restricted port for those who want one, not the only LAN path |
| First run | The `/setup` wizard is **open to the first visitor** while no household exists (D-073): no address guard, no setup code. Creating the first adult account closes it for good (`POST /setup/*` → 409). The install guide says to finish setup right after `docker compose up` |
| User auth | Sessions live in our own `session` table (household- and member-aware, so RLS and child-session revocation work), behind a custom tower-sessions store. Username and password (argon2id, OWASP parameters). Adults need 12+ characters, children 4+, and adults set up child accounts. Rolling 30 days for adults and 180 days for children. Login is rate-limited for each account and each IP, with lockout. An adult can reset a child's password and revoke a child's sessions, which is audited (D-031) |
| Session cookie | `HttpOnly; SameSite=Lax`, built with the `cookie` crate. `Secure` follows `cookie_secure = auto` (default): set when the request arrived over TLS, or from a trusted proxy with `X-Forwarded-Proto: https`; `true` and `false` force it. No HSTS is ever sent, because HTTP is a supported mode |
| Client IP | Login rate limiting keys on the client IP. **Trusted proxies default to loopback only** (D-069). The compose network has a fixed subnet and the Tailscale sidecar a fixed address, which compose sets as `FA_TRUSTED_PROXIES`; a hoster's own proxy is added in Settings, System. For trusted peers the rightmost `X-Forwarded-For` entry is the client and `X-Forwarded-Proto` is believed; for everyone else both headers are ignored and the peer address is the client |
| CSRF | SameSite=Lax, plus a required `X-FA-Request: 1` header on mutating requests (the JS client sets it) |
| Dashboard device | Paired by an adult (the UI, or `device pair`) → a long random token, of which only the SHA-256 hash is stored. It is sent as a bearer token from localStorage. The pairing link is `/dashboard?device=<token>` on the stored public address (and the tailnet address when the sidecar is on, and the D-064 address when that listener is on): the page saves the token and strips it from the URL at once, request logs record only the path, and every response sends `Referrer-Policy: no-referrer`. Revocable |
| PIN step-up | `POST /api/v1/device/elevate {member, pin}` → a `DeviceElevation` for that adult lasting 5 minutes. 5 failures lock it for 15 minutes and raise a notification to adults (D-019). Allowed on the main listener over HTTP or HTTPS (D-072) |
| Authorization | One `Policy::authorize(actor, action, resource)` in `fa-auth`, where actor is a user, a device, or a device elevated by an adult. It follows the matrix in Domain.md and is unit-tested exhaustively. Postgres RLS adds household isolation |
| RLS mechanics | Every transaction runs `SET LOCAL ROLE fa_app` (a NOLOGIN role without BYPASSRLS, because the Postgres login user may be a superuser) and sets either `app.household_id` (`tx_for`) or `app.system=on` (`tx_system`, used only for pre-household lookups, setup, and system tables such as `instance_setting`). Data is never queried through the raw pool. All SQL is parameterised |
| Secrets | **Envelope encryption.** A 256-bit master key comes from a file (default `/run/fa-secrets/master_key` in the `fa-secrets` volume, created by `init`, D-074; a hoster-provided docker secret still works). Each secret gets its own random DEK, and the DEK is wrapped by the master key (AES-256-GCM). `key_version` supports rotation (runbook `rotate-key.md`). The master key is **never** in the DB, git, or DB backups. The wizard requires an adult to copy or download it (`GET /api/v1/system/master-key`, audited) and Settings, System warns until a backup is confirmed |
| OAuth (Google) | The client ID and secret are runtime settings (Settings, System, the secret envelope-encrypted, or env, D-075). **Google requires an HTTPS redirect URI** (except localhost), so Google sign-in works only behind the Tailscale sidecar or another HTTPS proxy; on HTTP the UI shows it as "needs HTTPS". Testing-mode re-auth every 7 days (D-007) |
| GPU agent | A per-node token is issued at pairing time. The agent connects outbound to the core API only |
| Supply chain | `cargo deny` checks advisories and an AGPL-compatible licence allowlist. `pnpm audit` and license-checker do the same for the web app. Sidecar images are pinned by version or digest. We don't redistribute model weights, and their licences are recorded in the catalogue (D-025, D-035) |
| Public-repo hygiene | A `gitleaks` scan in CI. A check rejects real-looking email domains in `config/examples/` and fixtures. Docs and logs must never contain household specifics (D-004, D-035) |

## 6. Configuration layers
Three layers, each with one owner and one place to edit it (D-065, D-075):

| Layer | What | Where it lives | Edited by |
|---|---|---|---|
| 1. **Infrastructure** | `DATABASE_URL` (or its parts plus `FA_DATABASE_PASSWORD_FILE`), `FA_SECRETS_DIR` / `FA_MASTER_KEY_PATH`, `FA_BIND_ADDR`, `FA_HTTP_PORT`, `FA_HTTP_HOST_IP`, `FA_LOG_LEVEL`, `FA_LOG_FORMAT`, `FA_WEB_DIST_DIR`, `TS_AUTHKEY`, `TS_HOSTNAME` | environment (compose defaults; `.env` optional) | the hoster; needs a restart |
| 2. **Runtime settings** | public address (`public_base_url`, `tailnet_base_url`), `cookie_secure` (`auto\|true\|false`), ntfy URL and token, the ntfy address for phones (`ntfy_public_url`; empty derives the tailnet `:8443` or this host's `:8090`), Google client ID and secret, plain-HTTP feeds, private hosts for feeds (review C-05), the D-064 listener fields, trusted proxies (D-069), `key_backup_confirmed_at` | `instance_setting` (system-scoped table: `key text PK, value jsonb, secret_id → secret NULL, updated_at`); secrets envelope-encrypted | adults in **Settings, System**; the wizard fills the first values |
| 3. **Household data** | members, sources, lists, rules, pipelines, and so on | per-household tables (RLS) | adults in the UI |

- **Resolution for layer 2:** environment variable > DB > default. A field set by the environment is shown read-only as "set by the server", so every existing `FA_*` variable keeps working and overrides the UI.
- **Reload:** a change publishes `pg_notify('fa_settings')`, and the process re-reads the settings handle that every consumer (notify client, OAuth, pairing links, proxy trust) reads through. Listener and bind changes say "restart needed".
- **Public address:** the wizard stores the browser's `location.origin`; links produced without a request (notification click URLs) use the stored value only.
- **Household TOML (D-018)** is no longer part of setup. Import (upload with a dry-run preview) and export (download) are in Settings, System, through `POST /api/v1/config/import` and `GET /api/v1/config/export`, the same code as `fa-server config import|export`. Secrets are excluded from export.

## 7. Jobs & scheduling
- `job` table: `kind, payload jsonb, run_at, priority, attempts, max_attempts, locked_by, locked_until, idempotency_key UNIQUE`. Workers claim jobs with `FOR UPDATE SKIP LOCKED` and retry with exponential backoff. Jobs that keep failing are marked `dead` and shown on the status page.
- `schedule` table: cron expression, job kind, enabled. The worker ticks every 30 s and enqueues due jobs (deduplicated by idempotency key).
- **M0 jobs:** session cleanup and a `backup.verify` check that looks at the age of the newest dump file in `backup_dir` (mounted read-only into the worker).
- **M1 adds** `calendar.sync_due` (every minute), `calendar.sync{source}`, `calendar.prune` (daily), `weather.fetch` (`*/30`), `weather.evaluate` (after each fetch), and `weather.retention` (daily). See [Feeds.md](Feeds.md).
- **M3 adds** `mail.retention` (daily), which applies the configurable content policy, marks learning examples, and expires stuck pending messages (D-034). It also adds `mail.refetch`, which loads purged content from Gmail on demand for eval, dry-run, and re-processing. That job is rate-limited to the Gmail quota and keeps content in memory only.

## 8. Deployment (page "Deployment")
We ship **`deploy/compose.yaml`**, which has every service (D-033, D-065). The default install is: download `compose.yaml`, `docker compose up -d`, open `http://<server>:8080`, finish the wizard. Nothing in `.env` is required. **How it is updated is up to the hoster**; we provide versioned images, migrations that run on start and are backward compatible within a minor version, and release notes.

| Profile | Service | Image | Notes |
|---|---|---|---|
| default | `init` | `ghcr.io/jonganz/fa-server:<tag>` | One-shot `fa-server init`: creates the master key and DB password in volume `fa-secrets` if absent (D-074). Others wait with `service_completed_successfully` |
| default | `postgres` | `pgvector/pgvector:pg16` | `POSTGRES_PASSWORD_FILE` from `fa-secrets`. Volume `pgdata`. Not published to the host |
| default | `app` | `ghcr.io/jonganz/fa-server:<tag>` | `serve all` (D-076). Publishes `${FA_HTTP_HOST_IP:-0.0.0.0}:${FA_HTTP_PORT:-8080}`, and `${FA_LAN_DASHBOARD_HOST_IP:-127.0.0.1}:8081` for the opt-in D-064 listener. Mounts `fa-secrets` read-only and the backup directory read-only (for `backup.verify`) |
| default | `ntfy` | `binwiederhier/ntfy:v2.17.0` | Published on `${FA_NTFY_PORT:-8090}`; reached by `app` as `http://ntfy:80` on the compose network |
| default | `backup` | `prodrigestivill/postgres-backup-local:16` | Nightly, keeping 7 daily, 4 weekly, and 6 monthly. Writes to volume `fa-backups` by default; `BACKUP_DIR` points it at a NAS mount instead (D-005) |
| `tailscale` | `tailscale` | `tailscale/tailscale:v1.104.2` | `TS_AUTHKEY`, `TS_HOSTNAME` (default `family`), `TS_STATE_DIR` on volume `ts-state`, `TS_USERSPACE=true`, `TS_SERVE_CONFIG` from the inline compose config `ts_serve` (443 → `app:8080`, 8443 → `ntfy:80`). Fixed address `172.30.89.10` on the compose subnet, which compose passes to `app` as `FA_DEFAULT_TRUSTED_PROXIES` (a default, editable in Settings, System; D-069) |
| `ai-cpu` | `s1-small`, `embed` | pinned sidecar images | M4 (D-025, D-026) |
| `gpu` | `ollama`, `s1-large` | pinned, NVIDIA runtime | On a GPU node, or on a single machine that runs everything (D-026, D-033) |

- **Images (D-077):** `.github/workflows/release-image.yml` builds `linux/amd64` with buildx on `v*` tags and pushes `ghcr.io/jonganz/fa-server` (public package). arm64 is a later CI matrix change; until then arm64 hosts build locally.
- **Local build:** `compose.build.yaml` adds `build:` from the checkout (`docker compose -f compose.yaml -f compose.build.yaml up -d --build`), for developers and non-amd64 hosts. No split api/worker override is shipped (D-076).
- **Tailscale on the host** (v3): still possible as an advanced alternative (`tailscale serve` on the host to `127.0.0.1:8080`, with `FA_TRUSTED_PROXIES` set to the Docker gateway). WoL from the core node (M4.1) needs host networking or a host-level helper, decided in AI.md.
- The image is multi-stage: cargo-chef builds the Rust binaries, and node builds `apps/web`. The result is a debian-slim runtime with the web bundle at `/srv/web`.
- **GPU node:** `docker compose --profile gpu up ollama s1-large`. `fa-gpu-agent` runs **natively** from the release binary and `deploy/systemd/fa-gpu-agent.service`, because it needs host process visibility, `nvidia-smi`, and `systemctl suspend` (D-020).

## 9. Backups & restore
- The DB is backed up nightly as described above. The worker role raises an alert if the newest dump in the backup directory is older than 26 hours.
- Backups contain whatever email content the retention policy keeps. Under `keep_for_learning` that is the bounded set of learning examples. Email bodies are not encrypted at rest, and the backup location is trusted (D-034, superseding D-030).
- **The master key is not in the DB backups and must be backed up separately and offline** (password manager or printed). The wizard requires an adult to copy or download it, and Settings, System warns until a backup is confirmed (D-074). Without it, the encrypted secrets (OAuth tokens, app passwords) are unrecoverable, although that only means re-authorising each account.
- **The `fa-secrets` volume** holds the master key and the DB password. Losing the volume with no key backup means re-adding every sign-in; a lost DB password is recovered by restoring into a fresh Postgres with a new one.
- `deploy/runbooks/restore.md`: stop `app` → `pg_restore` into a fresh DB → put the master key back into `fa-secrets` → start → `/readyz`. **It is rehearsed once during M0**, and again on the M3.5 install.

## 10. Dev workflow
- `just dev` brings up Postgres in compose, then runs `cargo watch` for `serve all` and `pnpm dev` (with Vite proxying to the api).
- `just test` runs nextest and vitest. `just e2e` runs Playwright against compose. `just gen` regenerates the TS types and OpenAPI client.
- Seed data comes from `config/examples/household.example.toml` (a fictional family). No real data goes in fixtures (D-004).
- **CI (GitHub Actions, D-032):**
  - `ci.yml`: fmt, clippy `-D warnings`, nextest with a Postgres service, cargo deny (advisories and licences), gitleaks, the example-data check, sqlx offline check, and web lint, typecheck, vitest, and build.
  - `release-image.yml` (on `v*` tags): the `linux/amd64` `fa-server` image to **public** GHCR (D-077), plus the `fa-gpu-agent` binary as a release asset.

## 11. M0 definition of done
1. The workspace builds, and GitHub Actions `ci.yml` is green: fmt, clippy, tests, cargo deny, and the web checks.
2. Migrations exist for Identity, Audit, Secret, job, and schedule, with RLS on.
3. `config import` of the example household works, and `config export` round-trips it.
4. Login, logout, and sessions work. The RBAC policy has tests. Device pairing and PIN elevation work.
5. The Vue shell has login, a member list, and a `/dashboard` placeholder with a device token, an SSE heartbeat, and the keep-alive measures.
6. `deploy/compose.example.yaml` (default profile) comes up on the core node, `tailscale serve` is reachable from a phone, a backup lands on the NAS, and the restore rehearsal passes.

_M0 as built (v3): `compose.example.yaml` and host `tailscale serve`. The M3.5 equivalent of item 6 is the D-065 install: `docker compose up -d`, the wizard, the sidecar reachable from a phone (plan/M3.5.md)._
