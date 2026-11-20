---
id: T-02
title: "One shared modal on the dashboard"
milestone: M5.3
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/dashboard/Modal.vue", "apps/web/src/dashboard/TaskFlow.vue", "apps/web/src/dashboard/details/RecipeDetail.vue", "apps/web/src/dashboard/modal.test.ts", "apps/web/src/dashboard/tasks.test.ts", "apps/web/src/dashboard/food.test.ts"]
decisions: []
---

# T-02: One shared modal on the dashboard

You are a sub-agent on milestone M5.3 of the Family Assistant project (Vue 3 + TypeScript PWA in `apps/web`; the kiosk is the wall-screen dashboard).

User request: "The 'Add Task' modal on the desktop should use the same modal component used elsewhere on the dashboard. If the others aren't already a shared component, ensure they are."

## Read first
- `plan/_rules-common.md`, `plan/M5.3/_rules.md`, `plan/M5.3.md`
- `apps/web/src/dashboard/Modal.vue` (the shared modal: backdrop, title bar, close, idle close, `fullscreen` prop), its users (`details/EventDetail.vue`, `DayDetail.vue`, `WeatherDetail.vue`, `Whiteboard.vue`, `timers/AddTimerModal.vue`: read-only)
- `dashboard/TaskFlow.vue` (hand-rolled `.modal` for `complete-picker` and `add-form`), `dashboard/details/RecipeDetail.vue` (hand-rolled `role="dialog"` teleported full screen, with a scale row in its header), `views/DashboardView.vue` (how details and modals are mounted; read-only)

## You own
The paths in `owns`. Don't edit the other `details/*`, `timers/*`, `DashboardView.vue`, `strings.ts` or `ui/**` (report needs).

## Work
1. Render the Add task form and the "who did it" picker (and the "who adds" picker inside the add flow) inside `Modal` (title from existing strings: `S.addTask`, `S.whoDidIt`, `S.whoAdds`), keeping every `data-testid` (`add-form`, `complete-picker`, `add-title`, …) reachable, Esc/close, submit and cancel behaviour, and the idle close only where it does not lose typed text unexpectedly (if the idle close would discard a half-typed form, extend `Modal` with an opt-out prop such as `idle-close` default true and use false for the add form).
2. Render the recipe window inside `Modal` with `fullscreen`, keeping its content, the scale row (put it where the modal's header allows: add a named `#actions` slot to `Modal` if needed, default empty), `data-testid="recipe-detail"` and its tick behaviour. Remove the now-duplicate backdrop, header, close and Teleport CSS from both files. If `Modal` is `position: absolute` inside the view, keep the recipe window covering the same area as today.
3. Update tests that assert on the old markup; add a test that the add form and the recipe window render a `Modal` (`[role=dialog]` inside the shared backdrop) and close through it.
4. Take care of look: the Add task form must look like the other popups (same radius, backdrop, title bar), and its inputs/buttons stay 48 px high and sized from the kiosk text.

## Acceptance
- [ ] Web checks from `_rules.md` green
- [ ] Report: what changed, new props/slots on `Modal`, anything outside your paths that needs changing
