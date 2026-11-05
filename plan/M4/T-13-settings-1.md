---
id: T-13
title: "Migrate Settings I (personal, household, members, devices, about), login and setup to the design system"
milestone: M4
wave: 3
depends_on: [T-05]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/SettingsIndexView.vue", "apps/web/src/views/settingsNav.ts", "apps/web/src/views/settingsNav.test.ts", "apps/web/src/views/SettingsAccountView.vue", "apps/web/src/views/SettingsAppearanceView.vue", "apps/web/src/views/appearance.test.ts", "apps/web/src/views/SettingsHouseholdView.vue", "apps/web/src/views/SettingsMembersView.vue", "apps/web/src/views/DevicesView.vue", "apps/web/src/views/SettingsAboutView.vue", "apps/web/src/views/aboutModel.ts", "apps/web/src/views/about.test.ts", "apps/web/src/views/LoginView.vue", "apps/web/src/views/SetupView.vue", "apps/web/src/layouts/SetupLayout.vue", "apps/web/src/views/OAuthDoneView.vue", "apps/web/src/components/PinPad.vue", "apps/web/src/components/PinPad.test.ts", "apps/web/src/components/HuePicker.vue", "apps/web/src/components/huePicker.test.ts", "apps/web/src/stores/household.ts", "apps/web/src/stores/members.ts", "apps/web/src/stores/session.ts", "apps/web/src/stores/setup.ts", "apps/web/src/views/settings1.test.ts", "apps/web/src/themes/**", "apps/web/src/views/setup/**", "apps/web/src/views/parts/MembersManager.vue", "apps/web/src/views/parts/MemberAccess.vue", "apps/web/src/views/parts/memberAccess.test.ts", "apps/web/src/views/parts/memberForm.ts", "apps/web/e2e/ui-shots.routes/settings1.ts"]
decisions: [D-095, D-096, D-070, D-073, D-074, D-075, D-081, D-088]
---

# T-13: Migrate Settings I (personal, household, members, devices, about), login and setup to the design system

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The design system (`apps/web/src/ui/`, T-02) and the new app shell with two reference pages (T-05) are in place and were reviewed by the user at checkpoints R1 and R2. You migrate the Settings index and navigation data, the Personal and Household settings pages, Devices and About, plus **Login**, the **first-run wizard** and the OAuth landing page. Five tasks migrate views in parallel in this wave, so stay strictly inside your paths.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Navigation*, *View ownership* (what is **not** yours), *Test files*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4 and §2a (wizard), §3c (Settings area), §3d (Appearance)
- **The reference pages** from T-05: `apps/web/src/views/TasksView.vue` and `apps/web/src/views/SettingsHouseholdView.vue`, the "rules of the house" in T-05's report (quoted by the main session under *R2 feedback* below), and `apps/web/src/ui/` with `UiGallery.vue`
- `apps/web/e2e/*.spec.ts`: grep for the labels, roles and test ids your views use

## You own
The paths in `owns` above. `SettingsHouseholdView.vue` is T-05's reference page: keep it, adjust only if `settingsNav.ts` changes require it. `PinPad` is also used by the kiosk dashboard: keep its kiosk look (prop or wrapper) and run the dashboard tests. **Not yours:** `ui/**`, `app/**`, `layouts/AppLayout.vue`, `router/**`, `gen/**`, `dashboard/**`, `kiosk/**`, and every view, component, store and test file of another wave-3 task (see *View ownership* in `plan/M4.md`).

## Work
1. **`settingsNav.ts` and `SettingsIndexView`:** add an `icon: IconName` and a one-line `summary` to every page entry, and keep the groups and guards (§3c; the **AI** group's entry is added by T-15 in wave 4, so do not add it). The index is a grouped `UiList` (icon, title, summary, chevron) that `SettingsLayout` (T-05) renders as the list pane.
2. **Account:** a password card (form with Save bar) and a sessions card (list, revoke with confirm). **Appearance:** theme family, variant and mode as instant choices (`UiSegmented` for mode, a grid of theme swatches as cards) with a live preview; there is no Undo needed beyond picking again, but keep the snackbar consistent. **Members:** a list of members (avatar, name, role chip) and a member editor in a `UiSheet` on compact, or a detail pane, with cards for Profile, Login and PIN, Aliases and tags, and Interests (forms with the Save bar); the hue picker (`HuePicker`) restyled as a swatch row. Second-adult rules (D-088) are unchanged. **Devices:** paired devices as cards; pairing in a `UiSheet` with the link and QR code; the layout editor stays as it is inside a card (it belongs to `dashboard/`). **About:** cards for server and bundle versions, with the stale-bundle banner as a warning card.
3. **Login:** a centred card on wider screens and full-width on compact; password field with a show and hide `UiIconButton`.
4. **Setup wizard:** `SetupLayout` becomes a stepper (step n of 7 with `UiProgress`), each step one card, Next and Back as `UiButton`s, field errors under fields; the master-key step keeps Copy and Download and the required checkbox (D-074). **OAuth done:** a result card with continue.
5. **Applying changes (D-096):** every switch, stand-alone single choice and list checkbox goes through `useInstantSave` (Undo snackbar). Every multi-field form goes through `useFormDraft` with `UiSaveBar`. Creating goes through a form in a `UiSheet`. Destructive actions confirm in a `UiDialog` unless they have Undo. Remove the page's own Save logic, toasts and banners in favour of these.
6. **Mobile-first layout:** a `UiPage` scaffold with `UiTopAppBar` (title, back on nested pages, actions). Collections are a `UiList` on compact and cards or a list in a card on wider screens. Each subject gets a `UiCard`. Empty, loading and error states use `UiEmptyState`, `UiSkeleton` and `UiErrorState`. Icons come from `ui/icons.ts` only.
7. **No page-local colours or ad-hoc kit classes:** only `--ui-*` tokens and `ui/` components. Remove uses of `PageHeader`, `StateBlock`, `ErrorBanner` and `Tabs` from your files (do not delete those components; the main session does after wave 3), and the old `base.css` kit classes (`.form`, `.card-list`, `.badge` and so on) from your templates.
8. **Keep behaviour:** guards, store calls, SSE invalidation, `If-Match` and 409 handling, and every existing feature of your views stay. This is a visual and interaction revamp, not a feature change. Keep the accessible names and test ids that `apps/web/e2e` uses; any change you cannot avoid goes in the report.
9. **Tests:** update your area's test file (`apps/web/src/views/settings1.test.ts`, split by the main session) and the tests next to the components and stores you own. Add at least one instant-save Undo test and one dirty-form test where your views have them.
10. Run `scripts/ui-shots.sh --all` for your routes (add them to the route constant in `apps/web/e2e/ui-shots.spec.ts` only if the main session made it data-driven per area; otherwise list the routes in the report).

## R1 items (user, 2026-11-04)
- **Appearance:** add a **Density** control (`UiSegmented`: Comfortable / Compact) next to the theme picker. It is instant, uses `getDensity` and `setDensity` from `src/app/density.ts` (T-05), and is stored per device.
- **Theme data:** the accent and on-accent pair is below 4.5:1 in Solarized light, Tokyo Night day and Rosé Pine dawn (T-02's contrast test). Fix it without changing the theme's identity: adjust only the `on-accent` value (or, if that cannot reach 4.5:1, darken the accent at most one step toward the palette's own darker shade). Drop those three from any allow-list in `ui/tokens.test.ts` by reporting the change. You own `apps/web/src/themes/**` for this.

## R2 feedback
The user answered R2 on 2026-11-04 and 2026-11-05. These rules apply to every wave-3 page:
- **Shell (T-18, done):** phones get a menu button in the top bar that opens a modal drawer, with no bottom bar. Put contextual actions in `UiTopAppBar`'s `#actions` slot (at most two icon buttons plus an overflow `UiMenu` at 360 px). The desktop drawer collapses to the rail. Your page must use `UiPage` + `UiTopAppBar` (Settings children: `SettingsPage`), or phones get no menu button.
- **Save bar:** `UiSaveBar` is sticky at the bottom on every width. Use it for every multi-field form (D-096).
- **Density:** comfortable is the default; build for it, and check the compact density in the gallery does not break your page.
- **Collections on phones:** **cards on every width** (not a plain list on compact). `ResponsiveItems` should render a single-column card stack on compact.
- **Log out** stays at the foot of the navigation drawer. Do not add it to Settings → Account.
- Follow T-05's "rules of the house" and T-18's outcome (`plan/M4/T-05-app-shell.md`, `plan/M4/T-18-shell-r2.md`).
- **Account:** no Log out button here (it is in the drawer).

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass; `corepack pnpm@9 run format` was run
- [ ] `grep -nE "var\(--color-|#[0-9a-fA-F]{3,6}\b" <your .vue files>` shows no page-local colours (theme hues for member colours through `var(--hue-*)` are allowed)
- [ ] e2e selectors you changed are listed (do not run `scripts/e2e.sh`; the main session does)
- [ ] screenshots taken (count from the script's output) and the routes covered
- [ ] report:
  - the files changed;
  - components or variants missing from `ui/` (and the workaround used meanwhile, inside your files);
  - store changes made;
  - questions for the user's R3 review;
  - "list what outside your paths needs changing"
