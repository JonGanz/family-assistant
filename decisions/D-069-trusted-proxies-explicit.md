---
id: D-069
title: Trusted proxies: loopback by default, plus an explicit list that the compose file sets
status: accepted
date: 2026-10-26
source_questions: [Q-064]
supersedes: 
superseded_by:
affects: [arch/Platform.md]
---

# D-069: Trusted proxies: loopback by default, plus an explicit list that the compose file sets

## Context
Review finding C-04: `X-Forwarded-For` was trusted from all of `172.16.0.0/12`, so any container on the bridge or a 172.16/12 home LAN could spoof client IPs past the login limiter.

## Decision
Option A. The default `FA_TRUSTED_PROXIES` (and the Settings, System field, D-075) is loopback only. The compose file gives the network a fixed subnet and the Tailscale sidecar a fixed address, and sets `FA_TRUSTED_PROXIES` to that address. `X-Forwarded-Proto` is read from trusted proxies only.

## Consequences
- With no proxy (plain HTTP on the LAN, D-072) the peer address is the client IP, which is correct.
- Hosters with their own reverse proxy add its address in Settings, System.

## Alternatives considered
- B, keep 172.16.0.0/12.
- C, detect the Docker gateway at startup.
