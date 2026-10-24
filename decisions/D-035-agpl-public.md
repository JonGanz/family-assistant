---
id: D-035
title: Public repository and images under AGPL-3.0-or-later
status: accepted
date: 2026-10-23
source_questions: [Q-033]
supersedes: 
superseded_by:
affects: [arch/Platform.md]
---

# D-035: Public repository and images under AGPL-3.0-or-later

## Context
The user picked option B (public, AGPL-3.0). They have kept the docs free of personal information; the only personal detail in the repo is the reference hardware.

## Decision
- The GitHub repo and GHCR images are **public**. `LICENSE` holds the AGPL-3.0 text. Cargo and package metadata use `license = "AGPL-3.0-or-later"`.
- `cargo deny` keeps a licence allowlist that is compatible with AGPL (MIT, Apache-2.0, BSD, ISC, MPL-2.0, Zlib, Unicode). Copyleft-incompatible or non-commercial licences are rejected.
- Third-party model weights and sidecar images are **not** distributed by us. Hosters pull them, each under its own licence. The model catalogue (D-026) records each licence and flags restricted ones (for example GLiDE's community licence, or Strands' training-data note).
- **Privacy checks in CI** keep D-004 true now that everything is public:
  - a `gitleaks` secret scan;
  - a check that nothing in `config/examples/` uses a non-example email domain (only `example.com`, `example.org`, and similar).

## Consequences
- The design logs and decisions are public. That's fine, but they must never contain household specifics (this rule is now in CLAUDE.md).
- A public GHCR means hosters need no token.

## Alternatives considered
Permissive licences (MIT or Apache), or keeping the repo private. Not chosen.
