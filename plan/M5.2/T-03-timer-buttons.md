---
id: T-03
title: "Kiosk timer card buttons"
milestone: M5.2
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/dashboard/timers/TimerCard.vue", "apps/web/src/dashboard/timers/TimersView.vue", "apps/web/src/dashboard/timers/AddTimerModal.vue", "apps/web/src/dashboard/timers/timers.test.ts"]
decisions: []
---

# T-03: Kiosk timer card buttons

You are a sub-agent implementing one task of milestone M5.2 of the Family Assistant project (Vue 3 + TypeScript PWA in `apps/web`; the kiosk is the wall-screen dashboard).

## Problem
The user, after v0.1.34 tried to fix it: "the buttons are still inconsistent with the UI. They don't properly fit their text and the alignment is weird." The timer card (`dashboard/timers/TimerCard.vue`) uses hand-rolled `.a` buttons in two flex rows (`+1 min` / Start-Pause-Resume, then Reset / Remove) with `flex: 1 1 auto`, `white-space: nowrap`, a viewport-clamped font, and a taller first row, inside cards of a responsive grid (`TimersView.vue`). Causes to check: nowrap labels overflowing narrow cards, rows with different heights and widths, `flex: 1 1 auto` making unequal widths, the icon-only Remove sitting alone in a row with Reset, and styling that differs from the shared buttons.

## Read first
- `plan/_rules-common.md`, `plan/M5.2/_rules.md`; `arch/Frontend.md` *Timers view*
- `apps/web/src/ui/UiButton.vue`, `UiIconButton.vue` and their tokens (the shared button look, radius, heights), how other kiosk views style buttons (`dashboard/details/RecipeDetail.vue` scale row, `dashboard/Whiteboard.vue` tool buttons, `AddTimerModal.vue` `.btn`)
- `apps/web/src/dashboard/timers/*`, `apps/web/src/dashboard/strings.ts` (read-only)

## You own
The paths in `owns`. Don't edit `ui/**` or `dashboard/strings.ts` (report needs).

## Work
1. Use the shared `UiButton` (`filled` for the primary action: Start/Resume/Dismiss; `tonal` for +1 min, Pause and Reset) and `UiIconButton` for Remove, unless they cannot reach the kiosk's sizes; then match their radius, padding, typography and colours exactly with tokens.
2. Layout: one consistent grid. Suggested: a 2-column grid of equal-width buttons of one height (primary action and +1 min; Reset and Remove), labels that never overflow (allow wrapping or shrink via container query units so "+1 min", "Resume", "Dismiss" fit at the smallest card width), icon and label centred and vertically aligned, no row taller than another. Use container queries on the card so sizing follows the card, not the viewport.
3. Keep `data-testid`s, behaviour and the ringing style. Make the Add-timer button and the modal's buttons consistent with the same look.
4. Test in `timers.test.ts` that the actions still render and emit per state.

## Acceptance
- [ ] Web checks from `_rules.md` green
- [ ] Report: what changed, the smallest card width you designed for, anything outside your paths that needs changing (the main session takes screenshots at kiosk sizes)
