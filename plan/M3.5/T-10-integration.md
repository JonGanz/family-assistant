---
id: T-10
title: "Integration: e2e, compose smoke, CI"
milestone: M3.5
wave: 5
depends_on: [T-02, T-03, T-04, T-05, T-06, T-07, T-08]
agent: {model: sonnet, effort: high}
status: todo
owns: ["e2e/**", "apps/web/e2e/**", "scripts/**", ".github/workflows/ci.yml", "deploy/compose.example.yaml", "deploy/compose.dev.yaml"]
decisions: [D-065, D-068, D-069, D-072, D-073, D-074, D-078]
---

# T-10: Integration: e2e, compose smoke, CI

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md` *Done means* (your checklist), all contracts
- `scripts/{e2e,compose-smoke,ci-local}.sh`, `e2e/`, `apps/web/e2e/`, `.github/workflows/ci.yml`
- the reports of T-02 to T-08 (added below)

## You own
- `e2e/**`
- `apps/web/e2e/**`
- `scripts/**`
- `.github/workflows/ci.yml`
- `deploy/compose.example.yaml`
- `deploy/compose.dev.yaml`

## Work
1. **e2e (API, `e2e/`):**
   - `setup.spec.ts` against an api instance on an **empty** database. Add a second instance to `scripts/e2e.sh` with its own DB and port 18094; the main suite keeps its CLI-seeded household. It covers the full wizard API, 409s after completion, and `setup_required` before it.
   - Cookie behaviour: no `Secure` over plain HTTP; `Secure` with `X-Forwarded-Proto: https` from loopback.
   - Removed routes 404; the device inbox gets 403; adult-scoped approvals (two adults, an owners-only source and a household source); the dashboard `pending_approvals`.
   - Advisories clear when the weather stub stops matching.
   - Settings PATCH → restart → new value.
2. **UI e2e (`apps/web/e2e/`):**
   - the wizard end to end on the empty instance (plain HTTP origin, which also proves the insecure-context path);
   - Settings, System;
   - the dashboard has no inbox and no dismiss, and shows the members badge;
   - a child can't reach `/settings/members`, and `/members` is gone.
3. **`scripts/compose-smoke.sh`:** rewrite for `deploy/compose.yaml` + `deploy/compose.build.yaml`, **with no `.env`**:
   - project `fa-smoke`, ports 18180/18190;
   - check `init` (file modes and owners, idempotent on a second `up`);
   - `/readyz`, then setup through the API;
   - login cookie without `Secure`;
   - `docker compose --profile tailscale config` validates;
   - finish with `down -v`.
   Remove `deploy/compose.example.yaml` and every script and CI reference to it.
4. **CI:**
   - `ci.yml` runs the new smoke where it ran the old one;
   - validate `release-image.yml` with a YAML parse step;
   - `scripts/ci-local.sh` matches.
5. Throwaway containers: `docker rm -f -v`. Leave 0 dangling volumes.

## Acceptance
- [ ] `scripts/ci-local.sh`, `scripts/e2e.sh` and `scripts/compose-smoke.sh` pass; quote the spec and test counts from their output
- [ ] `docker volume ls -f dangling=true` is empty afterwards
- [ ] the report lists every *Done means* item and how it was verified, or why not (real-hardware steps belong to the shakedown)
