---
id: D-127
title: Reach tailnet AI hosts through the Tailscale sidecar's outbound proxy
status: accepted
date: 2026-11-20
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Platform.md]
---

# D-127: Reach tailnet AI hosts through the Tailscale sidecar's outbound proxy

## Context
User bug report of 2026-11-20: with the Tailscale sidecar, an Ollama host on the tailnet only works when entered by its LAN IP. The sidecar runs in userspace mode and only serves inbound HTTPS to the app; the app container is not on the tailnet and its LLM client has no proxy (R-009).

## Decision
The sidecar also runs tailscaled's outbound HTTP proxy, bound to its address on the compose network only (TS_OUTBOUND_HTTP_PROXY_LISTEN=172.30.89.10:1055, not published). The server reads FA_TAILNET_PROXY (default in the install file: http://tailscale:1055; empty turns it off) and the LLM client sends requests whose host is a MagicDNS name (*.ts.net), a tailnet IPv4 address (100.64.0.0/10) or a tailnet IPv6 address (fd7a:115c:a1e0::/48) through that proxy; every other destination, including other single-label names such as the ollama container, is dialled directly. No UI setting: the address decides.

## Consequences
Entering the tailnet name or 100.x address of the Ollama host in Settings > AI works when the tailscale profile is on. The proxy has no authentication: anything on the compose network can reach the tailnet through it, so keep the port unpublished and restrict the sidecar's node with Tailscale ACLs. Only the AI client uses it; calendars, feeds and recipe fetches are unchanged. Verified from Tailscale source (R-009) and by a local fake-proxy test; not verified against a live tailnet.

## Alternatives considered
Kernel-mode Tailscale with the app in the sidecar's network namespace (NET_ADMIN, /dev/net/tun, couples the app to the sidecar); Tailscale on the host (manual per install, MagicDNS not visible in containers); a per-endpoint proxy setting in the database (more UI for no gain); sending all app traffic through the proxy (breaks private-address policy and the ollama container name).
