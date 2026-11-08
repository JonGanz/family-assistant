---
id: D-123
title: The app and kiosk pick up a new version after a deploy
status: accepted
date: 2026-11-08
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/Platform.md]
---

# D-123: The app and kiosk pick up a new version after a deploy

## Context
On 2026-11-08 the user reported that, after updates, the UI often does not update. Mobile is the worst case, especially an installed PWA, where a forced refresh is hard. The user asked for a solution. This record comes from that instruction (the D-064 model).

Causes found in the code:
1. The server serves the web build through `ServeDir` with no `Cache-Control`, so browsers cache `index.html` (and the manifest) heuristically. Over plain HTTP, where no service worker runs (D-072), a phone can keep an old `index.html` whose hashed assets are gone after an update.
2. Over HTTPS the `autoUpdate` service worker installs a new version, but an open page keeps running the old JavaScript until it reloads. An installed PWA or the kiosk stays open for days.
3. Browsers check for a new service worker only on navigation, and a single-page app rarely navigates.

The server and the web app are built from the same commit (`FA_GIT_COMMIT`, `__APP_COMMIT__`), and Settings → About already compares them, but only on that page.

## Decision
- **Cache headers on the app shell:** `index.html`, `sw.js`, `registerSW.js` and `manifest.webmanifest` are sent with `Cache-Control: no-cache` (always revalidated). Hashed files under `/assets/` get `public, max-age=31536000, immutable`. Other static files (icons) get `max-age=3600`.
- **A new version is detected from the server, not guessed:**
  - every API response carries `X-FA-Build: <commit>`;
  - the live stream (`/api/v1/stream`) sends a `build` event `{commit}` when it connects, so a reconnect after a deploy reports the new build at once.
  - The client compares it with its own build (`__APP_COMMIT__`). An unknown build on either side, and the Vite dev server, never trigger an update.
- **Applying it:**
  - The service worker is asked to check (`registration.update()`), then the page reloads.
  - **App:** a snackbar "A new version is available · Reload". It reloads **automatically** the next time the app comes back to the foreground (a PWA resumed from the background) or on the next navigation, but never while a form has unsaved changes.
  - **Kiosk:** reloads automatically when no dialog is open and the device is not unlocked with a PIN, or as soon as that ends.
- **Update checks:** the service worker also checks for an update whenever the app returns to the foreground, and hourly while it stays open.
- **Escape hatch:** Settings → About gets **Reload app**. It removes the service worker's app caches (not the offline shopping list queue) and reloads, so a phone can always force-refresh without browser menus.

## Consequences
- `fa-api`: a cache-header layer on the static files, an `X-FA-Build` response header, and a `build` SSE event.
- Web: `lib/appUpdate.ts`; `http.ts` reads the header; a global unsaved-form count in `useFormDraft`; the kiosk store and `KioskLayout` reload rule; the About button.
- Frontend.md §1.1 and §5, Platform.md (static serving).
- A deploy reaches open phones without user action once they are foregrounded, and the kiosk within seconds.

## Alternatives considered
- `registerType: 'prompt'` alone: it covers only HTTPS and needs a tap.
- Polling `index.html` for a new hash: it needs a second mechanism over plain HTTP. The version header is cheaper and already on every request.
- Reloading at any time: it would lose typed text, so unsaved forms block the reload.
- A per-device setting choosing prompt or auto: not asked for, and the rule above already never interrupts editing. Easy to add later if wanted.
