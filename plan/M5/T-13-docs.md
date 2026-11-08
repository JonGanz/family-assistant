---
id: T-13
title: "Docs: README Food section, roadmap, arch status"
milestone: M5
wave: 6
depends_on: [T-06, T-07, T-08, T-09, T-10, T-11]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["README.md"]
decisions: [D-108, D-109, D-112, D-113, D-114, D-115, D-117, D-118, D-119]
---

# T-13: README for M5

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted, public (AGPL) family organiser. M5 adds recipes, a meal plan with templates, and one living grocery list grouped by store and aisle, plus a Food view on phones and the kitchen kiosk. `README.md` is the entry point for people using and contributing to the project. It must describe M5 and stay correct.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md` (*Done means*, *API*, *Out of scope*); `arch/Meals.md`; `arch/Frontend.md` §3f.
- The T-02 to T-10 reports and the code, for the actual behaviour.
- `README.md`: the whole file, especially the feature sections, the AI setup section and the Roadmap.

## You own
`README.md` only.

## Work
1. **A "Food" section** (users):
   - importing recipes: link, paste and type;
   - what the LLM does: only for pages without schema.org data, and for proposing ingredient matches that an adult confirms;
   - images stored locally;
   - ratings;
   - the meal plan and templates (the "Fall" example, written generically);
   - making the shopping list;
   - staples;
   - stores and aisles;
   - offline shopping (which parts work over plain HTTP, and which need HTTPS);
   - the kiosk Food view and meals panel;
   - Settings → Meals.
   No household specifics (D-035).
2. **AI section:** the new LLM purpose `ingredient` and the recipe import route `POST /api/v1/recipes/import`, with `/ai/recipes/extract` as an alias for one milestone.
3. **`allow_private_hosts`:** recipe and image fetches now honour it, like calendars do. Say so wherever the README explains that setting.
4. **Roadmap:** the M5 row reads "✅ code-complete ([plan](plan/M5.md))" with a short scope. Keep M4.2 and M4.1 after it.
5. **Verify:** follow every command you changed or added, and the Food walkthrough, against the dev stack the main session starts for you. Never run `scripts/dev-run.sh` yourself and never bind 8080 or 5173: ask in the report if no stack is available. Note anything that does not match.

## Acceptance
- [ ] `python3 tools/fa.py check` passes.
- [ ] Every link in the new text resolves (relative paths exist).
- [ ] The report lists the sections changed, what was verified by running it, and what was not.
