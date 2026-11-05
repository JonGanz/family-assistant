---
id: T-12
title: "Migrate Mail, Message, Inbox, Labelling and Bills to the design system"
milestone: M4
wave: 3
depends_on: [T-05]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/MailView.vue", "apps/web/src/views/MessageView.vue", "apps/web/src/views/InboxView.vue", "apps/web/src/views/LabelView.vue", "apps/web/src/views/BillsView.vue", "apps/web/src/components/mail/**", "apps/web/src/stores/mail.ts", "apps/web/src/stores/mail.test.ts", "apps/web/src/stores/inbox.ts", "apps/web/src/stores/inbox.test.ts", "apps/web/src/stores/bills.ts", "apps/web/src/stores/bills.test.ts", "apps/web/src/views/mail.test.ts", "apps/web/src/views/parts/inbox/**", "apps/web/src/views/parts/message/**", "apps/web/e2e/ui-shots.routes/mail.ts"]
decisions: [D-095, D-096, D-057, D-060, D-062, D-063, D-078]
---

# T-12: Migrate Mail, Message, Inbox, Labelling and Bills to the design system

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The design system (`apps/web/src/ui/`, T-02) and the new app shell with two reference pages (T-05) are in place and were reviewed by the user at checkpoints R1 and R2. You migrate the adult mail area: **Mail** (`/mail`), **Message** (`/mail/:id`), **Inbox** (`/inbox`), **Labelling** (`/mail/label`) and **Bills** (`/bills`). Five tasks migrate views in parallel in this wave, so stay strictly inside your paths.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Navigation*, *View ownership* (what is **not** yours), *Test files*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4 and §3b (mail screens)
- **The reference pages** from T-05: `apps/web/src/views/TasksView.vue` and `apps/web/src/views/SettingsHouseholdView.vue`, the "rules of the house" in T-05's report (quoted by the main session under *R2 feedback* below), and `apps/web/src/ui/` with `UiGallery.vue`
- `apps/web/e2e/*.spec.ts`: grep for the labels, roles and test ids your views use

## You own
The paths in `owns` above. **Not yours:** `ui/**`, `app/**`, `layouts/AppLayout.vue`, `router/**`, `gen/**`, `dashboard/**`, `kiosk/**`, and every view, component, store and test file of another wave-3 task (see *View ownership* in `plan/M4.md`).

## Work
1. **Mail:** filters as `UiChip` filter chips (category, member, source, state) in a scrolling row, plus a toggle for ignored categories. `MessageRow` becomes a three-line `UiListItem` (sender as overline, subject as headline, snippet or chips as supporting, time trailing, time-sensitive marker as an icon). 'Load more' as a text button.
2. **Message:** a card for metadata and actions (open in Gmail, make task, mail actions in a `UiMenu`, Undo list), a card for the body (child-filter sections collapsible, D-055), a card for decisions (`DecisionFix` as `UiChip`s, one tap to fix), and a card for derived items.
3. **Inbox:** `UiSegmented` or tabs for events, bills, tasks and contacts with counts; each proposal is a `UiCard` (filled) with its editable fields inline (calendar picker as `UiSelect`, title and time as fields) and Approve (filled) and Reject (text) actions. Approve and reject are instant with Undo where the API allows it, otherwise they confirm. Keep the badge count in the store (T-05's navigation reads it). **Leave room for a provenance chip** in the card header (T-16 adds the AI chip in wave 4): a `header-trailing` area that renders nothing yet.
4. **Labelling:** phone-first, one message per screen in a card, the four decision points as chips, a confirm-all `UiButton` filled, `UiProgress` toward the sample size, the 'in eval sample' toggle as a `UiSwitch` (instant).
5. **Bills:** a list grouped by state (open, proposed, paid); each bill a `UiListItem` with payee, amount (tabular figures) and due date, and the pay task's state as a chip.
6. **Applying changes (D-096):** every switch, stand-alone single choice and list checkbox goes through `useInstantSave` (Undo snackbar). Every multi-field form goes through `useFormDraft` with `UiSaveBar`. Creating goes through a form in a `UiSheet`. Destructive actions confirm in a `UiDialog` unless they have Undo. Remove the page's own Save logic, toasts and banners in favour of these.
7. **Mobile-first layout:** a `UiPage` scaffold with `UiTopAppBar` (title, back on nested pages, actions). Collections are a `UiList` on compact and cards or a list in a card on wider screens. Each subject gets a `UiCard`. Empty, loading and error states use `UiEmptyState`, `UiSkeleton` and `UiErrorState`. Icons come from `ui/icons.ts` only.
8. **No page-local colours or ad-hoc kit classes:** only `--ui-*` tokens and `ui/` components. Remove uses of `PageHeader`, `StateBlock`, `ErrorBanner` and `Tabs` from your files (do not delete those components; the main session does after wave 3), and the old `base.css` kit classes (`.form`, `.card-list`, `.badge` and so on) from your templates.
9. **Keep behaviour:** guards, store calls, SSE invalidation, `If-Match` and 409 handling, and every existing feature of your views stay. This is a visual and interaction revamp, not a feature change. Keep the accessible names and test ids that `apps/web/e2e` uses; any change you cannot avoid goes in the report.
10. **Tests:** update your area's test file (`apps/web/src/views/mail.test.ts`, split by the main session) and the tests next to the components and stores you own. Add at least one instant-save Undo test and one dirty-form test where your views have them.
11. Run `scripts/ui-shots.sh --all` for your routes (add them to the route constant in `apps/web/e2e/ui-shots.spec.ts` only if the main session made it data-driven per area; otherwise list the routes in the report).

## R2 feedback
The user answered R2 on 2026-11-04 and 2026-11-05. These rules apply to every wave-3 page:
- **Shell (T-18, done):** phones get a menu button in the top bar that opens a modal drawer, with no bottom bar. Put contextual actions in `UiTopAppBar`'s `#actions` slot (at most two icon buttons plus an overflow `UiMenu` at 360 px). The desktop drawer collapses to the rail. Your page must use `UiPage` + `UiTopAppBar` (Settings children: `SettingsPage`), or phones get no menu button.
- **Save bar:** `UiSaveBar` is sticky at the bottom on every width. Use it for every multi-field form (D-096).
- **Density:** comfortable is the default; build for it, and check the compact density in the gallery does not break your page.
- **Collections on phones:** **cards on every width** (not a plain list on compact). `ResponsiveItems` should render a single-column card stack on compact.
- **Log out** stays at the foot of the navigation drawer. Do not add it to Settings → Account.
- Follow T-05's "rules of the house" and T-18's outcome (`plan/M4/T-05-app-shell.md`, `plan/M4/T-18-shell-r2.md`).

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
