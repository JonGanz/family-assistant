---
id: T-03
title: "Import: section names, heading lines, plugin groups, LLM sections"
milestone: M5.1
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-recipe/**"]
decisions: [D-124]
---

# T-03: Import: section names, heading lines, plugin groups, LLM sections

You are a sub-agent implementing one task of milestone M5.1 of the Family Assistant project (self-hosted family organiser, Rust backend).

## Read first
- `plan/_rules-common.md`, `plan/M5.1/_rules.md`, `plan/M5.1.md` (Types: `DraftSection`, `RecipeDraft.sections`)
- `decisions/D-124-recipe-sections.md`, `arch/Meals.md` §4.1 2a
- `crates/fa-recipe/src/{lib.rs,jsonld.rs,microdata.rs,llm.rs,ingredient.rs,types.rs}`, `crates/fa-domain/src/recipe.rs`, the fixtures in `crates/fa-recipe/tests/fixtures/` (`graph.html` already has a `HowToSection`)

## You own
`crates/fa-recipe/**` only.

## Work
1. **JSON-LD step sections:** a `HowToSection` (an object with `itemListElement` and a `name`) becomes a `DraftSection` named after it with its steps; loose `HowToStep`s stay top-level (finish steps). Nested sections flatten into their outer section.
2. **Heading lines:** in any ingredient list (JSON-LD, microdata, LLM), a line with no parsed quantity, at most 60 chars, that ends with `:` or starts with "For the " (case-insensitive) starts a group named after it (strip `For the`, trailing `:`, trim; keep the original capitalisation of the rest, capitalise the first letter). Lines after it belong to the group until the next heading. Heading lines are never ingredients.
3. **Pairing:** normalise names (lower case, strip "for the", "make the", "to make the", "the", trailing ":" and punctuation, singularise a trailing "s"); an ingredient group and a step section with equal normalised names merge into one `DraftSection`. Order: step-section order, with each unpaired ingredient group inserted before the first section that follows it in page order (simplest stable rule is fine; document it). Unpaired groups and sections stay as sections of their own.
4. **Plugin ingredient groups** (only when JSON-LD has no headings): read the ingredient groups of common recipe-plugin markup, for example WP Recipe Maker and Tasty Recipes. **Verify the class names** against the plugin's public source or documentation (WebFetch on the plugin's wordpress.org or GitHub pages) before you rely on them; if you cannot verify a plugin's markup, skip that plugin and say so in the report. Assign a JSON-LD line to a group when its text equals (after whitespace normalisation) a line in that group; lines not found stay shared.
5. **LLM schema:** add an optional `section: string|null` per ingredient and per step to the structured output in `llm.rs`; group consecutive items with the same section name into `DraftSection`s (pairing as in 3). Keep the prompt short and the schema valid for the existing endpoint.
6. Fictional fixtures (D-035) and tests: a JSON-LD page with two `HowToSection`s ("Chicken", "Sauce"), ingredient headings "For the chicken:" / "For the sauce:", and a loose final step → two sections each with lines and steps, and one finish step; heading lines not in any ingredient list; a page with only step sections → sections with steps only and all lines shared; a plugin-markup page (for each verified plugin); an LLM stub test with sections.

## Acceptance
- [ ] `cargo test -p fa-recipe` pass; `cargo clippy -p fa-recipe --all-targets -- -D warnings`; `cargo fmt --check`
- [ ] `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: files, results with counts from output, which plugin markups were verified (with the source URL) and which were skipped, the ordering rule, and anything outside your paths that needs changing
