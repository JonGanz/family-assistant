---
id: Q-072
title: How do households get the app: a pulled image or a local build?
status: resolved
batch: 18
area: infra
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-077]
---

# Q-072: How do households get the app: a pulled image or a local build?

## Question
Today the guide clones the repository and builds the image on the server (`docker compose up -d --build`): about 4 GB RAM, 15 GB disk and up to 30 minutes. No release image is published yet (`ghcr.io/<owner>/fa-server` in the compose file is a placeholder).

**How should installs get the image?**

## Options
- A. **CI publishes a multi-arch image (amd64, arm64) to GHCR on each release tag; the install is "download `compose.yaml`, `docker compose up -d`"**, with no git clone. Building from a checkout stays for developers (recommended). Needs a human step: enable GHCR publishing and make the package public.
- B. Keep building from a checkout, but make it one command.
- C. Both, with the guide leading on building.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option A, but only amd64 at this point in time.
