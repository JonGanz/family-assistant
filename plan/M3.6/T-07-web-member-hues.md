---
id: T-07
title: "Web: member hues everywhere"
milestone: M3.6
wave: 3
depends_on: [T-03, T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/components/**", "apps/web/src/stores/**", "apps/web/src/views/**", "apps/web/src/lib/**", "apps/web/src/dashboard/panels/**", "apps/web/src/dashboard/filter.ts", "apps/web/src/dashboard/tasksModel.ts"]
decisions: [D-080]
---

# T-07: Web: member hues everywhere

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.6/_rules.md`
- `plan/M3.6.md`: *Backend wire formats*, *Web theme contract*; `decisions/D-080-member-hue-colours.md`; `arch/Frontend.md` §3d "Member colours"
- the regenerated `apps/web/src/gen/**` (member `color` is now `Hue | null`)
- grep `color` in `apps/web/src` to find every use (`MemberChip`, `EventCard`, `lib/eventColors.ts`, `lib/types.ts`, `stores/taskMap.ts`, `views/shared.ts`, `views/parts/MembersManager.vue`, `views/SetupView.vue`, `views/setup/validate.ts`, `dashboard/panels/*`, `dashboard/filter.ts`, `dashboard/tasksModel.ts`)

## You own
The paths in the front matter, **except** the files T-04 and T-06 own (`themes/**`, `styles/**`, `main.ts`, `App.vue`, `dashboard/theme.ts`, `dashboard/DashboardCanvas.vue`, `LayoutEditor.vue`, `editor.ts`, `layout/**`, `strings.ts`, `SettingsAppearanceView.vue`, `SettingsLayout.vue`, `SettingsIndexView.vue`).

## Work
1. A small helper in `lib/hues.ts`: `hueVar(hue: Hue | null | undefined) → 'var(--hue-blue)'` (unset is blue) and `isHue(x)`.
2. Every place that draws a member's colour (chips, avatars' ring or fallback, agenda and week event stripes, task items, member columns, routine panel, the task map) uses `hueVar`, not a string from the API. `eventColors`/`stripeStyle` fall back from the event's own **calendar colour** (hex, from outside) to the member's hue variable; a merged event may mix hex and variables. Update their tests.
3. **Colour text on a hue fill** uses the theme (`--color-bg` or `--color-text`) where it used `textColorFor` on a hex; keep `lib/contrast.ts` for calendar hex colours only.
4. **Member picker:** `MembersManager.vue` and the setup wizard (owner and family steps) replace the hex input with eight swatches drawn from the current theme (`var(--hue-*)`), accessible (a radio group, names as labels). `views/setup/validate.ts` validates a hue name instead of `#rrggbb`; the owner request sends a `Hue`.
5. Remove any remaining literal colours in components (except data-driven calendar/category/task-list colours, which stay as they are).
6. Fix and extend the tests (components, taskMap, eventColors, setup validation, a MembersManager test for the picker).

## Acceptance
- [ ] `corepack pnpm@9 run typecheck`, `run lint`, `test` pass; counts quoted only from the output
- [ ] `grep -rnE "#[0-9a-fA-F]{6}" apps/web/src --include=*.vue` lists only data-driven or test content (give the list in the report)
- [ ] report: any component that still needs a colour decision

## Outcome (T-07 report)
- `lib/hues.ts` (`hueVar`, `isHue`, `textOn`), `components/HuePicker.vue` (eight radio swatches) used in `MembersManager.vue` and the setup `OwnerStep.vue`; every member colour goes through `hueVar`; owner validation uses `isHue`; two literal fallbacks removed.
- View models keep `color: string`, now holding the CSS value from `hueVar`.
- Gap: no `MembersManager` test (it needs a mocked members store); `HuePicker` and the setup validation have their own tests.
- Left as hex on purpose (data-driven): the default colour of a new mail category and a calendar's colour input fallback, and the task-list default `#2f6fed` in `listForm.ts`.
- Main session fixed a fixture slip of its own in `dashboard/tasks.test.ts`.
