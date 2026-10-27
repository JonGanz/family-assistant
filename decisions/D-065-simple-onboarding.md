---
id: D-065
title: Simple onboarding: one compose file, optional Tailscale sidecar, optional HTTPS, first-run wizard
status: accepted
date: 2026-10-26
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Platform.md, arch/Frontend.md, arch/Overall.md, README.md, deploy/README.md]
---

# D-065: Simple onboarding: one compose file, optional Tailscale sidecar, optional HTTPS, first-run wizard

## Context
The user found the deployment and setup far too complex (2026-10-26, direct instruction, not a question): Tailscale and `tailscale serve` on the host, a required HTTPS address, a hand-made master key and `.env`, a household TOML file, and `docker compose exec api fa-server ...` calls to import it and create logins. They asked to pause after M3 and fix onboarding first.

## Decision
1. **Deployment is a docker-compose file.** Tailscale is an **optional sidecar** container (a compose profile), for households that want access from outside the home. Tailscale on the host stays a documented alternative, not the default.
2. **HTTPS is optional, not required.** The app runs over plain HTTP on the home network out of the box. HTTPS comes from the Tailscale sidecar (or any reverse proxy the hoster runs). Nothing refuses to start because the address is `http://`.
3. **First access runs an onboarding wizard** when nothing is configured yet: household, first adult and login, other members, access and notifications, master-key backup. The household TOML file and the `fa-server` admin commands stop being part of setup. TOML import and export (D-018) stay as an optional upload and download in the UI; the CLI stays for recovery and scripting.
Details (exposure on the LAN, first-run protection, secrets, which settings move into the UI, container layout, image distribution) are batch 18, Q-067 to Q-072.

## Consequences
- Amends D-010 (Tailscale on the host, HTTPS via `tailscale serve`) and widens D-064 (the dashboard-only LAN listener): the full app can now be on the home network over plain HTTP, so D-064 becomes an optional hardening mode rather than the only LAN path. Q-067 settles the default.
- Over plain HTTP, passwords, PINs and session cookies cross the home network unencrypted, and the browser treats the page as an insecure context: no service worker (so no PWA install or offline shell), no `crypto.subtle`, no `crypto.randomUUID`, no async clipboard. Google sign-in still needs HTTPS (Google refuses `http://` redirect URIs except localhost), so it is available only with Tailscale or another HTTPS proxy.
- D-031 (weaker child passwords) relied on the tailnet boundary; on a plain LAN that argument is weaker, so the UI says so.
- Implemented as milestone M3.5 Onboarding ([plan/M3.5.md](../plan/M3.5.md)) before M4.

## Alternatives considered
- Keep host Tailscale and required HTTPS, and only write a better guide: rejected by the user as still too complex.
- An installer script instead of compose: rejected; the user asked for docker-compose.
