---
id: T-03
title: "Weather card opens anywhere; Today button on the week view"
milestone: M5.3
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/dashboard/panels/WeatherPanel.vue", "apps/web/src/dashboard/panels/PanelHost.vue", "apps/web/src/dashboard/DashboardCanvas.vue", "apps/web/src/dashboard/strings.ts", "apps/web/src/dashboard/weekModel.ts", "apps/web/src/dashboard/weekModel.test.ts", "apps/web/src/dashboard/weatherHourly.test.ts", "apps/web/src/dashboard/dashboard.test.ts"]
decisions: []
---

# T-03: Weather card opens anywhere; Today button on the week view

You are a sub-agent on milestone M5.3 of the Family Assistant project (Vue 3 + TypeScript PWA in `apps/web`; the kiosk is the wall-screen dashboard).

User requests: (a) "Update the dashboard Home view so that clicking ANYWHERE on the weather card opens the weather modal." (b) "Add a today button to the dashboard week view to bring the user back to the current week in case they navigated far away."

## Read first
- `plan/_rules-common.md`, `plan/M5.3/_rules.md`, `plan/M5.3.md`
- `dashboard/panels/WeatherPanel.vue` (the inner `<section>` has the click that calls `open()`), `dashboard/panels/PanelHost.vue` (the card that wraps panels: padding and trim live here), `dashboard/DashboardCanvas.vue` (`weekOffset`, the `weekbar` with previous/next buttons and the "hide past" switch), `dashboard/strings.ts`, `weekModel.ts`

## You own
The paths in `owns`. Don't edit other panels or `ui/**`.

## Work
1. Weather: the whole card, including its padding and the advisory list, opens the weather detail (`detail` event with the same payload as today) when weather data exists; keyboard (Enter/Space) and `role="button"`/`tabindex` stay accessible on the card element. Do it so the click target is the card itself (e.g. the card element in `PanelHost` forwards a click, or the weather panel root is made to fill the card), without making other panels clickable. No behaviour change when there is no weather data.
2. Week bar: add a **Today** button (string in `strings.ts`, e.g. `S.weekToday = 'Today'`) next to the next-week chevron, sized like the other bar buttons and the kiosk text. It sets `weekOffset = 0`; it is disabled (or visually inactive and `aria-disabled`) when `weekOffset === 0`. Keep the bar fitting without wrapping at 1024×600 portrait and landscape (the e2e "every kiosk view fits common screen sizes" test checks buttons stay on screen).
3. Tests: weather click on the padding area (the card element) emits the detail request; the Today button resets the offset and is inactive at 0.

## Acceptance
- [ ] Web checks from `_rules.md` green
- [ ] Report: what changed, anything outside your paths that needs changing
