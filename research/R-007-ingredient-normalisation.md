---
id: R-007
title: Ingredient normalisation, unit conversion and seed data for the grocery list
date: 2026-11-07
method: Sonnet 5.5 sub-agent. Read crates/fa-recipe/src/ingredient.rs, jsonld.rs, arch/AI.md §9, D-015, D-021. Crates found with `cargo search` in /tmp/claude-1000/r7 (no repo Cargo files touched); version, licence and release date read from the crates.io API (`/api/v1/crates/<name>` and `/versions`) on 2026-11-07. Dataset licences from fetched pages (Open Food Facts terms of use, Wikidata licensing page, FDC download page and a search that surfaced the FDC API Guide text). Unit factors and aisle taxonomies are standard reference knowledge, not fetched
verified: Verified from primary sources = crate names, versions, licences, release dates (crates.io API); Open Food Facts ODbL/DbCL/CC BY-SA statement (terms-of-use page); Wikidata CC0 (licensing page). FDC "CC0 1.0 / public domain" was confirmed only via a search snippet quoting the FDC API Guide, and the FDC download and FAQ pages fetched did not themselves state the licence, so re-check before shipping. NOT verified by fetch (proposal or background knowledge) = pg_trgm licence and behaviour, the OFF taxonomy files' own licence header (none seen), whether FDC SR Legacy carries per-cup gram weights (it does in its portion table, from memory), unit factors, aisle lists, and every recommendation
informs: []
---

# R-007: Ingredient normalisation, unit conversion and seed data

## Current state (M4)
`fa-recipe::ingredient::parse_ingredient` returns `{text, quantity, unit, item}`. It handles ints, decimals, `1/2`, `1 1/2`, unicode fractions and ranges (low end), and a unit list: cup, tbsp, tsp, g, kg, ml, l, oz, lb, clove, can, pinch, slice. Gaps M5 must handle: no `fl oz`, pint, quart, gallon, stick, bunch, package; no `1 (14 oz) can` form; `item` keeps prep notes (`"onion, finely chopped"`, `"salt to taste"`), so it is not yet a matchable name. `jsonld.rs` reads JSON-LD only; `Recipe.import_method` already lists `microdata`.

## 1. Rust crates (crates.io API, 2026-11-07)
`scraper` and `strsim` are already workspace deps.

| Need | Crate | Version | Licence | Last release | Notes / fit |
|---|---|---|---|---|---|
| Plural | `pluralizer` | 0.6.0 | MIT OR Apache-2.0 | 2026-10-19 | Active, 3.6M downloads. Port of the JS `pluralize`: rule list plus irregulars and uncountables, `pluralize(word, 1, false)` singularises. Best fit; allows adding custom rules |
| Plural | `inflect-rs` | 0.26.2 | MIT | 2026-10-15 | Port of Python `inflect`; mostly generates plurals; 1.6k downloads, new. Weaker |
| Plural | `inflection-rs` | 0.2.2 | GPL-3.0-only | 2025-05-16 | Singularise and pluralise. GPL-3.0 is compatible with AGPL-3.0 (GPLv3 §13) but needless friction; skip |
| Plural | `Inflector` | 0.11.4 | BSD-2-Clause | 2019-01-20 | Unmaintained; to_singular is table-based on Rails rules (`tomatoes`, ok; `leaves`, `knives` shaky). Skip |
| Plural | `cruet` | 1.0.0 | BSD-2-Clause | 2026-04-03 | Maintained fork of Inflector, same Rails rules. Backup |
| Units | `uom` | 0.38.0 | Apache-2.0 OR MIT | 2026-02-14 | Compile-time typed SI quantities. Has volume and mass with US units (teaspoon, tablespoon, US cup, fluid ounce, pound), but a type-checked static system does not fit values parsed at runtime from text, and it has no `stick`, `pinch`, `clove`, and US/imperial cup nuances you must pick. Heavy generics for little gain. Not recommended |
| Units | `measurements` | 0.11.1 | MIT | 2025-10-18 | Simple runtime Volume/Weight, but no cooking units. Not needed |
| Units | `runits` | 0.1.10 | Apache-2.0 | 2026-10-29 | 61 downloads, too new. Skip |
| Microdata | `microdata` | 0.1.1 | MIT | 2025-05-31 | 1.4k downloads, one maintainer, parses itemscope/itemprop. Small, not proven |
| Microdata / RDFa | `schemaorg-validate` | 0.3.0 | MIT | 2026-05-29 | 347 downloads; validator oriented. Skip |
| Fuzzy | `strsim` | 0.11.1 | MIT | 2024-04-02 | Levenshtein, Jaro-Winkler, Sorensen-Dice; stable, 1.25B downloads; already used. Fit for catalogue matching in Rust |
| Fuzzy | `rapidfuzz` | 0.5.0 | MIT | 2023-12-01 | Faster, but stale. Not needed |
| Fuzzy (DB) | Postgres `pg_trgm` | contrib ext | PostgreSQL licence (from background knowledge, not fetched) | ships with PG | `similarity()`, `%`, GIN/GiST index. Good for searching a larger catalogue from SQL; needs `CREATE EXTENSION` in a migration (check the Docker image includes contrib; the official image does) |

**Microdata/RDFa:** no maintained, widely used crate found. Microdata is a small attribute grammar (`itemscope`, `itemtype`, `itemprop`, value from `content`/`datetime`/`href`/`src`/text). A hand parser over `scraper` (about 100 lines, emitting the same `Recipe` JSON shape the JSON-LD path already consumes) is better than adding a 1.4k-download crate. RDFa is rare on recipe sites; defer it.

Plural caveat for food: generic rules fail on `molasses`, `hummus`, `asparagus`, `couscous`, `leaves` (bay leaves), `halves`. Keep a small editable exceptions list (uncountable plus irregular) that wraps `pluralizer`, or lean on the alias table in the catalogue instead of singularising blindly.

## 2. Seed ingredient data

| Source | Licence (verified how) | Size | Aliases / plurals / categories | Shipping a derived subset in AGPL repo |
|---|---|---|---|---|
| USDA FoodData Central (Foundation, SR Legacy) | CC0 1.0 / US public domain, per the FDC API Guide as quoted by search; pages fetched did not state it, re-check | Foundation 9 MB JSON, SR Legacy 205 MB JSON (zipped 12 MB), Branded 3 GB | Nutrient-oriented descriptions (`"Salt, table"`), food categories (USDA groups, not aisles), no plurals; SR Legacy has per-portion gram weights | Fine. CC0 needs no attribution; credit anyway ("FoodData Central") |
| Open Food Facts taxonomies (`taxonomies/food/ingredients.txt`, `categories.txt`) | Database ODbL, contents DbCL, images CC BY-SA (terms-of-use page). The taxonomy file's own licence is not stated in the files seen (the server repo is AGPL-3.0). Treat as ODbL | `ingredients.txt` about 2.9 MB of text | Multilingual synonyms per entry (including plurals), parent links, some density in g/ml, Wikidata ids. No aisles | Possible but ODbL: attribution, and a derived database must be shared under ODbL. ODbL covers the *database*, and the licence says a Produced Work may have other terms, but a shipped seed table is a derived database. Compatible in spirit with an open repo, yet adds a second licence and NOTICE duty. Avoid unless needed |
| Wikidata | CC0 for structured data (licensing page) | Huge; needs SPARQL extraction | Labels and aliases per language, classes (`food ingredient`), no aisles | Fine legally; extraction noise is high. Use at most to harvest aliases offline |
| Hand-written seed | Ours (AGPL-3.0, author-owned) | 200 to 400 rows | Exactly the fields needed: canonical name, plural/alias list, aisle, default unit, staple default, optional density | Zero licence issue |

**Conclusion:** a hand-written seed of roughly 250 to 400 common ingredients with aisle, aliases and an optional density is the practical default: small, reviewable, editable in the UI (D-026, D-029), and licence-clean. Use FDC (CC0) only as a reference for density and portion weights; do not bulk-import OFF (ODbL share-alike) or Wikidata. Facts (names, aisle assignments) are not copyrightable in themselves, but a list copied wholesale from a database brings that database's licence.

## 3. Cooking units

| Unit | Metric equivalent | Notes |
|---|---|---|
| tsp | 4.92892 ml (US); commonly 5 ml | Metric spoon is exactly 5 ml |
| tbsp | 14.7868 ml (US); 15 ml metric | 1 tbsp = 3 tsp. Australia uses 20 ml |
| fl oz | 29.5735 ml (US) | 1 fl oz = 2 tbsp |
| cup | 236.588 ml (US customary); 240 ml US legal/labelling; 250 ml metric cup | 1 cup = 16 tbsp = 8 fl oz |
| pint | 473.176 ml (US, 2 cups); 568.261 ml (imperial, 20 imp fl oz) | Imperial differs by about 20 percent |
| quart | 946.353 ml (US, 2 pints); 1136.52 ml (imperial) | |
| gallon | 3785.41 ml (US, 4 quarts); 4546.09 ml (imperial) | |
| ml, l | 1 l = 1000 ml | |
| oz (weight) | 28.3495 g | Avoirdupois |
| lb | 453.592 g | 1 lb = 16 oz |
| g, kg | 1 kg = 1000 g | |

Proposal: three dimensions (volume, mass, count). Store canonical base units (ml, g, each) plus a household setting for display system (US, metric) and cup size (default US 240 ml or 236.588 ml; configurable per CLAUDE.md "configurable, not hard-coded"). Convert only within a dimension. Count units (clove, can, slice, bunch, stick) are their own dimension per ingredient and never convert to volume without a density or "each" weight.

**Volume to weight** needs per-ingredient density (g per ml or g per cup): e.g. flour about 120 g/cup, sugar about 200 g/cup, butter about 227 g/cup (values are common kitchen figures, not verified here). Open sources: USDA FDC SR Legacy portion table (gram weight per "1 cup" etc., CC0) and the OFF `density` property (ODbL). No other permissively licensed table was verified. Practical approach: optional `density_g_per_ml` column in the catalogue, filled in the seed for about 40 common dry goods and left blank otherwise; if blank, volume and mass lines for one ingredient are kept as separate lines rather than guessed.

## 4. Grocery category taxonomies (description only)
Grocery apps and store layouts commonly use 10 to 20 sections, ordered roughly as a store walks: Produce (fruit, vegetables, fresh herbs); Meat and poultry; Seafood; Deli; Dairy and eggs; Bakery and bread; Frozen; Pantry/dry goods (pasta, rice, grains, baking, canned goods, sauces, oils and vinegars, spices and seasonings, condiments); Breakfast/cereal; Snacks; Beverages; Household and cleaning; Personal care; Baby; Pet; Other. Variants split International/Ethnic, Baking, Canned, or Health. Proposal: the default list above as an editable, orderable table per household (sort order = shopping route), plus "Other" as the fallback.

## 5. Recommendation
1. **Pipeline:** parse line (extend `parse_ingredient`: more units, `1 (14 oz) can`, strip prep text after a comma and trailing "to taste") then **normalise name** (lowercase, ASCII fold, drop descriptors such as kosher, fresh, large, chopped from an editable stop list, singularise with `pluralizer` plus exceptions) then **match catalogue** (exact on canonical or alias, then strsim Jaro-Winkler or token-set with a threshold; or `pg_trgm` if the catalogue is searched in SQL) then **convert** to base unit within dimension then **merge** by (ingredient id, dimension) summing quantity and keeping per-meal sources (D-015).
2. Staples (D-021): match on the same normalised name; "kosher salt" resolves to `salt` by descriptor stripping plus alias, so a staple only needs the canonical row.
3. Unmatched lines are kept verbatim as free-text items, never dropped, and flagged for a one-tap "add to catalogue / pick match" in the UI; the choice is stored as a new alias so it is learned deterministically.
4. **LLM (7-8B local, structured JSON) only for unmatched lines**, in the existing `fa-recipe` LLM path: input line plus the nearest 5 catalogue candidates, output `{ingredient_id | null, canonical_name, quantity, unit, aisle}` with a confidence; low confidence goes to the human. Never auto-create catalogue rows without confirmation.
5. **Microdata:** hand parser over `scraper`, feeding the JSON-LD `Recipe` shape; skip RDFa.
6. **Seed:** hand-written TOML/SQL of about 300 ingredients: canonical name, aliases (including plurals), aisle, default unit, `is_staple` default (salt, pepper, oil and similar, household editable), optional density for about 40 dry goods. Ships under the repo's AGPL; FDC may be consulted for densities, with credit. No OFF/ODbL data in the repo.
7. **Crates to add:** `pluralizer` (MIT OR Apache-2.0); reuse `strsim` and `scraper`; no `uom`.
