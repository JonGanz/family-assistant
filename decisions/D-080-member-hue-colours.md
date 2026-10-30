---
id: D-080
title: Members choose a named hue; existing members migrate to blue
status: accepted
date: 2026-10-29
source_questions: [Q-074]
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/Domain.md]
---

# D-080: Members choose a named hue; existing members migrate to blue

## Context
A member's `color` was a free hex string. With themed palettes (D-079), a hue name lets each theme supply its own variant of that colour. The app has not been released, so there is no production data to preserve.

## Decision
- A member's `color` stores **one of eight hue names**: red, orange, yellow, green, cyan, blue, purple, pink. **No custom hex** (Q-074 answer: hue name only).
- A new migration sets every existing member to `blue`. The user chose that over mapping each hex to the nearest hue.
- The UI draws the colour from the current theme's hue (`var(--hue-<name>)`), so changing theme re-tints every member.
- **Category colours are unchanged.** ~~Calendars imported from Google or ICS keep their own hex colours.~~ Calendar colours were removed later (D-086).

## Consequences
- The DTO keeps the field name `color`, but its type becomes the `Hue` enum, so the generated API types change.
- The member picker shows the current theme's swatches.
- An event with no calendar colour falls back to its member's hue (`eventColors.ts`). A merged event can mix hex calendar colours and hue variables, which the striped border already allows.
- The existing migration is not edited; a new one converts the data.

## Alternatives considered
- Free hex colours (or hue plus optional custom hex): rejected by the user. Custom colours would not follow the theme.
- Mapping each existing hex to the nearest hue: not needed before release.
