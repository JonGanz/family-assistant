---
title: Platform
scope: code layout, runtime, security, deployment, backups, dev workflow
status: draft
updated: 2026-10-23
drawio: Platform.drawio
drawio_src: src/Platform.py
depends_on: [D-002, D-003, D-004, D-005, D-006, D-010, D-018, D-019, D-020, D-023, D-025, D-026]
milestone: M0
---

# Platform (DRAFT v1)

The platform that M0 builds and every later milestone depends on. Items marked **[Q-0xx]** depend on an open question.

## 1. Repository & workspace layout (page "Packages")
```
/                      (code repo, location is [Q-030])
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
│  ├─ fa-server        CLI: serve {api|worker|all} · migrate · config {import|export} · user {add|reset} · device {pair|revoke} · eval
│  └─ fa-gpu-agent     runs on GPU nodes (M4)
├─ apps/web            Vue 3 + TS + Vite + Pinia + vue-router + vite-plugin-pwa (pnpm)
│  └─ src/gen/         generated TS types (ts-rs) and API client (from OpenAPI)
├─ deploy/
│  ├─ core/compose.yaml   + .env.example, Dockerfile (cargo-chef + node build stages)
│  ├─ gpu/compose.yaml    (ollama, s1-large), fa-gpu-agent.service
│  └─ runbooks/           install.md, restore.md, rotate-key.md
├─ config/examples/    household.example.toml (fictional family, D-004)
└─ justfile            dev tasks
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
- One image, `fa-server`, runs as two containers:
  - **api:** HTTP, SSE, and serving the web bundle.
  - **worker:** scheduler and jobs.

  `serve all` runs both in one process for small installs and for dev.
- **Startup:** `migrate` runs from the api only, under a Postgres advisory lock. Then config validation runs, then (M4) `fa-sysprobe`.
- **Health:**
  - `/healthz` reports the process is alive.
  - `/readyz` reports the DB is reachable and migrations are current.
  - A **System status** page in the UI shows nodes, backends, sources, the last backup, and queue depth.

## 4. API conventions
- REST + JSON under `/api/v1`. OpenAPI is generated, and the TS client is generated from it.
- **SSE** at `/api/v1/stream` pushes typed change events (`task.updated`, `event.proposed`, `weather.updated`, ...). The dashboard and PWA use it to stay live, sending `Last-Event-ID` when they reconnect.
- Pagination is keyset (cursor), using UUIDv7 ordering. Mutable resources carry a `version` and require `If-Match`, so a conflict returns 409.

## 5. Security (page "Security")
| Area | Design |
|---|---|
| Network | No public exposure. `tailscale serve` (on the host) terminates HTTPS on `https://<host>.<tailnet>.ts.net` and proxies to `api:8080`, which binds to localhost or the compose network only (D-010) |
| User auth | Username and password (argon2id, OWASP parameters). Session cookie is `HttpOnly; Secure; SameSite=Lax`, rolling 30 days, stored in Postgres. Login is rate-limited for each account and each IP. An adult can reset another account's password, which is audited. Kids' login is **[Q-029]** |
| CSRF | SameSite=Lax, plus a required `X-FA-Request: 1` header on mutating requests (the JS client sets it) |
| Dashboard device | Paired by an adult (`device pair` CLI or the UI) → a long random token, of which only the SHA-256 hash is stored. It is sent as a bearer token from localStorage, with a URL-fragment fallback (D-011). Revocable |
| PIN step-up | `POST /api/v1/device/elevate {member, pin}` → a `DeviceElevation` for that adult lasting 5 minutes. 5 failures lock it for 15 minutes and raise a notification to adults (D-019) |
| Authorization | One `Policy::authorize(actor, action, resource)` in `fa-auth`, where actor is a user, a device, or a device elevated by an adult. It follows the matrix in Domain.md and is unit-tested exhaustively. Postgres RLS adds household isolation |
| Secrets | **Envelope encryption.** A 256-bit master key comes from a file (`/run/secrets/fa_master_key`, a docker secret, mode 0400). Each secret gets its own random DEK, and the DEK is wrapped by the master key (AES-256-GCM). `key_version` supports rotation (runbook `rotate-key.md`). The master key is **never** in the DB, git, or DB backups |
| OAuth (Google) | The client ID and secret are in process config (`.env`, not in git). The redirect URI is on the tailnet HTTPS hostname, which Google accepts because it is real HTTPS. Testing-mode re-auth every 7 days (D-007) |
| GPU agent | A per-node token is issued at pairing time. The agent connects outbound to the core API only |
| Supply chain | `cargo deny` (licenses and advisories), `pnpm audit`. Sidecar images are pinned by digest (D-025) |

## 6. Configuration layers
1. **Process config** (`.env` / `fa.toml`, owned by whoever administers the server): DB URL, bind address, master key path, OAuth client, tailnet hostname, log level, NAS path.
2. **Household config** (stored in the DB and edited in the UI): members, sources, lists, rules, pipelines, and so on. It can be imported and exported as TOML (D-018). Secrets are excluded from export.

## 7. Jobs & scheduling
- `job` table: `kind, payload jsonb, run_at, priority, attempts, max_attempts, locked_by, locked_until, idempotency_key UNIQUE`. Workers claim jobs with `FOR UPDATE SKIP LOCKED` and retry with exponential backoff. Jobs that keep failing are marked `dead` and shown on the status page.
- `schedule` table: cron expression, job kind, enabled. The worker ticks every 30 s and enqueues due jobs (deduplicated by idempotency key).
- **M0 jobs:** session cleanup, audit-log compaction (none, since the log is append-only), and a `backup.verify` check that reads the backup container's status file.

## 8. Deployment (page "Deployment")
**Core node, `deploy/core/compose.yaml`:**
| Service | Image | Notes |
|---|---|---|
| `postgres` | `pgvector/pgvector:pg16` | volume `pgdata`. Not published to the host |
| `api` | `fa-server` | `serve api`. Publishes `127.0.0.1:8080` for tailscale serve |
| `worker` | `fa-server` | `serve worker` |
| `ntfy` | `binwiper/ntfy` | `127.0.0.1:8090`, exposed through `tailscale serve` on its own path or port (D-010) |
| `backup` | `prodrigestivill/postgres-backup-local` | Nightly, keeping 7 daily, 4 weekly, and 6 monthly, to `/mnt/nas/family-assistant` (host NFS/SMB mount) (D-005) |
| `s1-small`, `embed` | sidecars | compose **profile `ai`**, M4 (D-025, D-026) |

- Tailscale runs **on the host**, not in a container, so that `tailscale serve` can reach localhost and WoL packets go out on the host LAN.
- The image is multi-stage: cargo-chef builds the Rust binaries, and node builds `apps/web`. The result is a distroless or debian-slim runtime with the web bundle at `/srv/web`.
- **GPU node, `deploy/gpu/`:** `compose.yaml` runs `ollama` and `s1-large` with the NVIDIA runtime. `fa-gpu-agent` runs **natively** as a systemd service, because it needs host process visibility, `nvidia-smi`, and `systemctl suspend` (D-020).

## 9. Backups & restore
- The DB is backed up nightly as described above. A `status.json` beside the dumps lets the worker raise an alert if the last backup is older than 26 hours.
- **The master key must be backed up separately and offline**, for example in a password manager or printed. Without it, the encrypted secrets (OAuth tokens) are unrecoverable, although that only means re-authorising each account.
- `deploy/runbooks/restore.md`: stop api and worker → `pg_restore` into a fresh DB → put the master key back → start → `/readyz`. **It is rehearsed once during M0**.

## 10. Dev workflow
- `just dev` brings up Postgres in compose, then runs `cargo watch` for `serve all` and `pnpm dev` (with Vite proxying to the api).
- `just test` runs nextest and vitest. `just e2e` runs Playwright against compose. `just gen` regenerates the TS types and OpenAPI client.
- Seed data comes from `config/examples/household.example.toml` (a fictional family). No real data goes in fixtures (D-004).
- CI and hosting are **[Q-030]**. The update mechanism is **[Q-031]**.

## 11. M0 definition of done
1. The workspace builds. CI runs fmt, clippy, tests, and cargo deny.
2. Migrations exist for Identity, Audit, Secret, job, and schedule, with RLS on.
3. `config import` of the example household works, and `config export` round-trips it.
4. Login, logout, and sessions work. The RBAC policy has tests. Device pairing and PIN elevation work.
5. The Vue shell has login, a member list, and a `/dashboard` placeholder with a device token, an SSE heartbeat, and the keep-alive measures.
6. `compose up` on the core node works, `tailscale serve` is reachable from a phone, a backup lands on the NAS, and the restore rehearsal passes.
