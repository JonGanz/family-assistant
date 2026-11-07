import sys; sys.path.insert(0, "tools")
from drawio import Page, write, COMP, EXT, TITLE, NOTE, FRAME, ENUM, cls, assoc

# Page 1: Components (UML 2 component diagram)
pc = Page("components", "Components", 1400, 760)
b, e = pc.box, pc.edge
b("Meals: recipes, meal plan, grocery (UML 2 component), DRAFT v0 (M5, D-015, D-021, D-108; open: Q-098 to Q-114)", 20, 10, 1200, 30, TITLE)
web = b("apps/web\nMeal plan · Recipes · Grocery · Settings → Meals · dashboard meals/grocery panels", 380, 60, 520, 60, COMP)
api = b("fa-api\n/recipes · /recipes/import · /ingredients · /meals/* · /meal-plan · /grocery", 380, 180, 520, 60, COMP)
srv = b("fa-server\nwiring · worker", 1000, 180, 220, 60, COMP)
mea = b("fa-meals\nNormaliser · RecipeService · MealPlanService · GroceryService (generate, reconcile)\njobs: grocery.archive · recipe.image · ingredient.llm", 380, 320, 520, 80, COMP)
rec = b("fa-recipe\nfrom_html (JSON-LD, microdata) · from_text · from_llm", 980, 320, 320, 60, COMP)
llm = b("fa-llm\nProvider (structured JSON)", 980, 450, 320, 50, COMP)
con = b("fa-connectors\nHttp (private hosts blocked)", 620, 470, 280, 50, COMP)
dom = b("fa-domain\nRecipeDraft (moved from fa-recipe) · meals types · authz", 40, 320, 280, 60, COMP)
st = b("fa-store\nrecipe* · ingredient · aisle · meal_slot · meal_plan_entry · grocery_list · grocery_item(_source) · meals_settings", 40, 470, 520, 60, COMP)
page = b("«external» recipe web page / image", 980, 600, 260, 50, EXT)
for s, t, l in [(web, api, "REST + SSE grocery.updated"), (api, mea, "services"), (srv, mea, "job handlers, cron"), (mea, rec, "import"),
                (mea, llm, "ingredient.llm"), (rec, llm, "fallback"), (mea, con, "image fetch"), (rec, con, "fetch_page"),
                (mea, st, "repos"), (mea, dom, "types"), (api, dom, "authz")]:
    e(s, t, l)
e(con, page, "HTTPS GET", False)
b("Deterministic first: parse → clean → singularise → alias / trigram match → convert within a dimension. The LLM only proposes matches for unmatched lines (R-007)", 40, 600, 520, 70, NOTE)

# Page 2: Model (UML 2 class diagram)
pm = Page("model", "Model", 1400, 900)
pm.box("Meals: data model (UML 2 class), DRAFT v0", 20, 10, 800, 30, TITLE)
rcp = cls(pm, "Recipe", ["title, source_url?, servings?, yield_text?", "prep/cook/total_minutes?", "import_method: jsonld|microdata|llm|manual", "image_path?, tags[], note?, favourite", "version"], 40, 70, 280)
ri = cls(pm, "RecipeIngredient", ["ord, raw_text", "qty?, unit?, item?, note?", "match_state: matched|unmatched|proposed"], 40, 300, 280)
rs = cls(pm, "RecipeStep", ["ord, text"], 380, 300, 180)
ing = cls(pm, "Ingredient", ["canonical_name, aliases[]", "is_staple, default_unit?", "density_g_per_ml?"], 40, 500, 280)
ais = cls(pm, "Aisle", ["name, ord"], 40, 700, 180)
slot = cls(pm, "MealSlot", ["name, ord", "time \"HH:MM\", visible"], 700, 70, 220)
mpe = cls(pm, "MealPlanEntry", ["date YYYY-MM-DD", "recipe? xor free_text?", "servings?, cook_member?", "version"], 400, 70, 240)
gl = cls(pm, "GroceryList", ["(one per household)"], 1000, 300, 220)
gi = cls(pm, "GroceryItem", ["text, qty?, unit?", "section: list|staples", "manual, added_by?, added_via_device?", "checked_at?, checked_by?, archived_at?", "version"], 700, 450, 280)
gis = cls(pm, "GroceryItemSource", ["qty?, unit? (contribution)"], 420, 470, 220)
ms = cls(pm, "MealsSettings", ["grocery_archive_hours = 24", "unit_conversion, ingredient_llm", "image_mode, …"], 1000, 70, 260)
mem = cls(pm, "Member", ["display_name, role"], 1000, 650, 200)
assoc(pm, rcp, ri, "1", "*", kind="composition"); assoc(pm, rcp, rs, "1", "*", kind="composition")
assoc(pm, ri, ing, "*", "0..1"); assoc(pm, ing, ais, "*", "0..1")
assoc(pm, mpe, rcp, "*", "0..1"); assoc(pm, mpe, slot, "*", "1"); assoc(pm, gl, gi, "1", "*", kind="composition")
assoc(pm, gi, ing, "*", "0..1"); assoc(pm, gi, gis, "1", "*", kind="composition"); assoc(pm, gis, mpe, "*", "1", "from meal")
assoc(pm, mpe, mem, "*", "0..1", "cook (Q-101)"); assoc(pm, gi, mem, "*", "0..1", "added_by")
pm.box("Invariants: an entry has exactly one of recipe and free_text; a staples item is never manual; generated qty = Σ sources (+ manual delta)", 40, 820, 700, 50, NOTE)

# Page 3: Sequence (UML 2): generate a grocery list from meals
ps = Page("sequence", "Generate sequence", 1400, 700)
ps.box("Meals: generate the list from planned meals (UML 2 sequence), DRAFT v0 (D-015, D-021)", 20, 10, 1000, 30, TITLE)
ps.box("sd grocery.generate", 20, 50, 1340, 620, FRAME)
lanes = ["web (Meal plan)", "fa-api", "fa-meals GroceryService", "Normaliser", "fa-store / Postgres"]
xs = {}
for i, n in enumerate(lanes):
    x = 50 + i * 260
    ps.box(n, x, 90, 220, 36, "html=1;whiteSpace=wrap;")
    ps.box("", x + 109, 126, 2, 520, "html=1;dashed=1;strokeColor=#999999;")
    xs[n] = x + 105
def msg(a, c, y, label, dashed=False):
    s = ps.box("", xs[a], y, 10, 10, "html=1;fillColor=#dae8fc;")
    t = ps.box("", xs[c], y, 10, 10, "html=1;fillColor=#dae8fc;")
    ps.edge(s, t, label, dashed)
msg("web (Meal plan)", "fa-api", 160, "POST /grocery/generate {entry_ids[]} or {from, to}")
msg("fa-api", "fa-meals GroceryService", 210, "generate(household, entries) (adult; authz)")
msg("fa-meals GroceryService", "fa-store / Postgres", 260, "load entries, recipe lines, existing sources (tx)")
msg("fa-meals GroceryService", "Normaliser", 310, "scale by entry/recipe servings · normalise · convert within dimension")
msg("fa-meals GroceryService", "fa-store / Postgres", 370, "staple → section staples · else merge into unchecked item or insert · insert sources (skip if present)")
msg("fa-meals GroceryService", "fa-api", 440, "change set (for Undo)", True)
msg("fa-api", "fa-store / Postgres", 490, "publish SSE grocery.updated")
msg("fa-api", "web (Meal plan)", 540, "200 change set · snackbar with Undo (D-096)", True)

write("arch/Meals.drawio", [pc, pm, ps])
