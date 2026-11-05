---
id: T-16
title: "Inbox and message AI provenance: the AI chip on LLM-made proposals and decisions"
milestone: M4
wave: 4
depends_on: [T-12, T-08]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/InboxView.vue", "apps/web/src/views/MessageView.vue", "apps/web/src/components/mail/**", "apps/web/src/stores/inbox.ts", "apps/web/src/stores/inbox.test.ts", "apps/web/src/views/mail.test.ts"]
decisions: [D-094, D-095, D-057, D-060, D-078]
---

# T-16: Inbox and message AI provenance: the AI chip on LLM-made proposals and decisions

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The backend now records where each inbox proposal came from (`extracted_by: "rules" | "llm"`, T-03) and records decisions made by the LLM stage with `stage_kind = "llm"` (T-06). The inbox and message screens were migrated to the design system in wave 3 (T-12), which left a `header-trailing` slot on each proposal card for this. You make AI-made items recognisable without changing how approval works.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *Domain types* (`ExtractOrigin`, `extracted_by` on inbox items), *`llm` stage and the `mail.llm` job*
- `decisions/D-094-m4-llm-extraction-slice.md`; `arch/Frontend.md` §1.2 and **§3e** (AI provenance); `arch/AI.md`
- `apps/web/src/gen/` (regenerated after T-08: the inbox item types with `extracted_by`, and the decision type, which should carry `stage_kind`), `apps/web/src/views/{InboxView,MessageView}.vue`, `apps/web/src/components/mail/*` (`DecisionFix`, `decisions.ts`), T-12's report

## You own
The paths in `owns` above. **Not yours:** `ui/**` (if a chip variant is missing, report it and compose from `UiChip` inside `components/mail/`), `gen/**`, every other view and store.

## Work
1. **`components/mail/AiChip.vue`:**
   - a `UiChip` (assist, non-interactive by default) with the `ai` icon, the text "AI" and the confidence as a whole percentage ("AI · 82%");
   - an accessible name "Suggested by the local language model, 82% confident";
   - a tooltip on hover and focus on medium and expanded; on compact, a tap opens a small `UiMenu` or `UiSheet` with the same sentence;
   - colour from tokens only (e.g. tonal primary container).
2. **Inbox:**
   - render `AiChip` in each proposal card's `header-trailing` when `extracted_by === 'llm'`, using the item's existing confidence field (name it from `gen/`);
   - add an "AI only" `UiChip` filter in the inbox's filter row (client-side filter over the loaded page; it is not persisted);
   - approve and reject behaviour is unchanged (D-057, D-060).
3. **Message screen:** in the decisions card, a decision with `stage_kind === 'llm'` shows `AiChip` next to its value, and `DecisionFix` keeps working (a fix records a human decision as today). If the decision DTO in `gen/` has no `stage_kind`, do not hand-write it: report it, and show nothing.
4. Derived items on the message screen (events, bills or tasks made from it) also show `AiChip` when their `extracted_by` is `llm`, if the DTO carries it (otherwise report it).
5. **Tests** (`views/mail.test.ts`, component test):
   - an `llm` item shows the chip with the rounded percentage;
   - a `rules` item shows none;
   - the AI-only filter;
   - the chip's accessible name;
   - an LLM decision on the message screen shows the chip.
6. Run `scripts/ui-shots.sh --all` for `/inbox` and one message, with at least one LLM-made item if the e2e seed has one. If it has none, say so and suggest a seed row for the main session.

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass; `corepack pnpm@9 run format` was run
- [ ] no hand-written API types (missing fields reported, not invented)
- [ ] screenshots listed (count from the script's output)
- [ ] report:
  - the files changed;
  - DTO fields that were missing;
  - the e2e seed suggestion;
  - questions for the final review R4;
  - "list what outside your paths needs changing"
