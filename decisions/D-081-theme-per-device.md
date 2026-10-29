---
id: D-081
title: The theme is chosen per device, with an app-shipped default and no household setting
status: accepted
date: 2026-10-29
source_questions: [Q-075]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-081: The theme is chosen per device, with an app-shipped default and no household setting

## Context
The user answered Q-075 with their own model (2026-10-29): each person picks a theme on their own device. The app ships a default, which is used until they pick one. The kiosk uses whatever its device is configured to use.

## Decision
- The theme is a **per-device** choice. There is **no household default** and no household setting for it.
- **App devices (phones, desktops):** the choice is stored on that device (`localStorage`), and any signed-in member, adult or child, can change it in Settings, Personal, Appearance. The app ships a default theme, used until a choice is made.
- **Kiosk and other dashboard devices:** the theme is part of the device's own settings (`device.settings.theme`), set by an adult in Settings, Devices, with the layout editor's live preview. The kiosk never picks it itself (D-019). A new device starts with the app default.
- The kiosk uses the **same catalogue** as the app, not a separate dark theme.

## Consequences
- The choice does not follow a person from one device to another. That is the cost of having no server-side setting, and was the user's intent.
- Replaces the kiosk's `theme: auto|light|dark` layout field. Its `auto` meaning stays for the kiosk: dark between sunset and sunrise (from the weather data), since a wall display's own light/dark setting is not reliable.
- The burn-in shift applies whenever the active variant is dark.

## Alternatives considered
- A household default with per-member overrides (the original recommendation): not chosen.
- Per-member choice stored on the server: not chosen. The user wants it per device.
