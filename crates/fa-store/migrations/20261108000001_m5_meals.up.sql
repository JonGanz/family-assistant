-- M5: recipes, ingredients, meal plan, templates and the living grocery list (plan/M5.md, arch/Meals.md §3).

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE aisle (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    name         text NOT NULL,
    ord          int NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX aisle_household_name_key ON aisle (household_id, lower(name));

CREATE TABLE grocery_store (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    name         text NOT NULL,
    ord          int NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX grocery_store_household_name_key ON grocery_store (household_id, lower(name));

CREATE TABLE grocery_store_aisle (
    store_id     uuid NOT NULL REFERENCES grocery_store(id) ON DELETE CASCADE,
    aisle_id     uuid NOT NULL REFERENCES aisle(id) ON DELETE CASCADE,
    household_id uuid NOT NULL REFERENCES household(id),
    ord          int NOT NULL,
    hidden       boolean NOT NULL DEFAULT false,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (store_id, aisle_id)
);

CREATE TABLE recipe (
    id            uuid PRIMARY KEY,
    household_id  uuid NOT NULL REFERENCES household(id),
    title         text NOT NULL CHECK (char_length(title) BETWEEN 1 AND 200),
    source_url    text,
    servings      int CHECK (servings BETWEEN 1 AND 100),
    yield_text    text,
    prep_minutes  int CHECK (prep_minutes >= 0),
    cook_minutes  int CHECK (cook_minutes >= 0),
    total_minutes int CHECK (total_minutes >= 0),
    import_method text NOT NULL DEFAULT 'manual' CHECK (import_method IN ('jsonld', 'microdata', 'llm', 'manual')),
    tags          text[] NOT NULL DEFAULT '{}',
    note          text,
    favourite     boolean NOT NULL DEFAULT false,
    created_by    uuid REFERENCES member(id) ON DELETE SET NULL,
    version       int NOT NULL DEFAULT 1,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX recipe_household_title_idx ON recipe (household_id, lower(title));
CREATE INDEX recipe_title_trgm_idx ON recipe USING gin (title gin_trgm_ops);

CREATE TABLE recipe_image (
    recipe_id    uuid PRIMARY KEY REFERENCES recipe(id) ON DELETE CASCADE,
    household_id uuid NOT NULL REFERENCES household(id),
    mime         text NOT NULL CHECK (mime IN ('image/webp', 'image/jpeg', 'image/png')),
    bytes        bytea NOT NULL CHECK (octet_length(bytes) <= 524288),
    sha256       text NOT NULL,
    source       text NOT NULL CHECK (source IN ('import', 'upload')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ingredient (
    id               uuid PRIMARY KEY,
    household_id     uuid NOT NULL REFERENCES household(id),
    canonical_name   text NOT NULL,
    aliases          text[] NOT NULL DEFAULT '{}',
    is_staple        boolean NOT NULL DEFAULT false,
    aisle_id         uuid REFERENCES aisle(id) ON DELETE SET NULL,
    default_store_id uuid REFERENCES grocery_store(id) ON DELETE SET NULL,
    default_unit     text CHECK (default_unit IN ('g', 'ml', 'count')),
    origin           text NOT NULL DEFAULT 'user' CHECK (origin IN ('seed', 'user', 'llm')),
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, canonical_name)
);
CREATE INDEX ingredient_name_trgm_idx ON ingredient USING gin (canonical_name gin_trgm_ops);
CREATE INDEX ingredient_aliases_idx ON ingredient USING gin (aliases);

CREATE TABLE recipe_ingredient (
    id            uuid PRIMARY KEY,
    household_id  uuid NOT NULL REFERENCES household(id),
    recipe_id     uuid NOT NULL REFERENCES recipe(id) ON DELETE CASCADE,
    ord           int NOT NULL,
    raw_text      text NOT NULL,
    qty           double precision,
    unit          text,
    item          text,
    note          text,
    ingredient_id uuid REFERENCES ingredient(id) ON DELETE SET NULL,
    match_state   text NOT NULL DEFAULT 'unmatched' CHECK (match_state IN ('matched', 'unmatched', 'proposed')),
    proposal      jsonb,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (recipe_id, ord)
);
CREATE INDEX recipe_ingredient_ingredient_idx ON recipe_ingredient (ingredient_id);

CREATE TABLE recipe_step (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    recipe_id    uuid NOT NULL REFERENCES recipe(id) ON DELETE CASCADE,
    ord          int NOT NULL,
    text         text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (recipe_id, ord)
);

CREATE TABLE recipe_rating (
    recipe_id    uuid NOT NULL REFERENCES recipe(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id) ON DELETE CASCADE,
    household_id uuid NOT NULL REFERENCES household(id),
    stars        smallint NOT NULL CHECK (stars BETWEEN 1 AND 5),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (recipe_id, member_id)
);

CREATE TABLE meal_slot (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    name         text NOT NULL,
    ord          int NOT NULL,
    time         time NOT NULL,
    visible      boolean NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE meal_plan_entry (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    date         date NOT NULL,
    slot_id      uuid NOT NULL REFERENCES meal_slot(id) ON DELETE RESTRICT,
    recipe_id    uuid REFERENCES recipe(id) ON DELETE SET NULL,
    free_text    text,
    servings     int CHECK (servings BETWEEN 1 AND 100),
    created_by   uuid REFERENCES member(id) ON DELETE SET NULL,
    version      int NOT NULL DEFAULT 1,
    ord          int NOT NULL DEFAULT 0,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    -- Deleting a recipe keeps its entries: RecipeRepo::delete first sets recipe_id = NULL and
    -- free_text = the title in one UPDATE, so the check holds (a bare DELETE of a planned recipe fails).
    CONSTRAINT meal_plan_entry_xor CHECK ((recipe_id IS NULL) <> (free_text IS NULL))
);
CREATE INDEX meal_plan_entry_date_idx ON meal_plan_entry (household_id, date);

CREATE TABLE meal_template (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    name         text NOT NULL,
    days         int NOT NULL CHECK (days BETWEEN 1 AND 366),
    note         text,
    version      int NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE meal_template_cell (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    template_id  uuid NOT NULL REFERENCES meal_template(id) ON DELETE CASCADE,
    day          int NOT NULL CHECK (day >= 1),
    slot_id      uuid NOT NULL REFERENCES meal_slot(id) ON DELETE CASCADE,
    recipe_id    uuid REFERENCES recipe(id) ON DELETE CASCADE,
    free_text    text,
    servings     int CHECK (servings BETWEEN 1 AND 100),
    ord          int NOT NULL DEFAULT 0,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT meal_template_cell_xor CHECK ((recipe_id IS NULL) <> (free_text IS NULL))
);
CREATE INDEX meal_template_cell_day_idx ON meal_template_cell (template_id, day);

CREATE TABLE grocery_list (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL UNIQUE REFERENCES household(id),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE grocery_item (
    id               uuid PRIMARY KEY,
    household_id     uuid NOT NULL REFERENCES household(id),
    list_id          uuid NOT NULL REFERENCES grocery_list(id) ON DELETE CASCADE,
    ingredient_id    uuid REFERENCES ingredient(id) ON DELETE SET NULL,
    text             text NOT NULL,
    qty              double precision,
    unit             text,
    qty2             double precision,
    unit2            text,
    store_id         uuid REFERENCES grocery_store(id) ON DELETE SET NULL,
    aisle_id         uuid REFERENCES aisle(id) ON DELETE SET NULL,
    section          text NOT NULL DEFAULT 'list' CHECK (section IN ('list', 'staples')),
    manual           boolean NOT NULL,
    added_by         uuid REFERENCES member(id) ON DELETE SET NULL,
    added_via_device uuid REFERENCES device(id) ON DELETE SET NULL,
    checked_at       timestamptz,
    checked_by       uuid REFERENCES member(id) ON DELETE SET NULL,
    archived_at      timestamptz,
    version          int NOT NULL DEFAULT 1,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX grocery_item_open_idx ON grocery_item (list_id) WHERE archived_at IS NULL;
CREATE INDEX grocery_item_ingredient_idx ON grocery_item (ingredient_id);

CREATE TABLE grocery_item_source (
    item_id        uuid NOT NULL REFERENCES grocery_item(id) ON DELETE CASCADE,
    entry_id       uuid NOT NULL REFERENCES meal_plan_entry(id) ON DELETE CASCADE,
    household_id   uuid NOT NULL REFERENCES household(id),
    qty            double precision,
    unit           text,
    qty2           double precision,
    unit2          text,
    generated_from date NOT NULL,
    generated_to   date NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (item_id, entry_id)
);
CREATE INDEX grocery_item_source_entry_idx ON grocery_item_source (entry_id);

CREATE TABLE meals_change (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    kind         text NOT NULL CHECK (kind IN ('generate', 'reconcile', 'template_apply', 'swap', 'move')),
    actor_member uuid REFERENCES member(id) ON DELETE SET NULL,
    actor_device uuid REFERENCES device(id) ON DELETE SET NULL,
    inverse      jsonb NOT NULL,
    expires_at   timestamptz NOT NULL,
    undone_at    timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE meals_settings (
    household_id         uuid PRIMARY KEY REFERENCES household(id),
    grocery_archive_hours int NOT NULL DEFAULT 24 CHECK (grocery_archive_hours BETWEEN 1 AND 720),
    ingredient_llm       boolean NOT NULL DEFAULT true,
    trigram_threshold    real NOT NULL DEFAULT 0.45 CHECK (trigram_threshold BETWEEN 0.2 AND 0.95),
    descriptors          text[] NOT NULL DEFAULT '{}',
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now()
);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'aisle', 'grocery_store', 'grocery_store_aisle', 'recipe', 'recipe_image', 'ingredient',
        'recipe_ingredient', 'recipe_step', 'recipe_rating', 'meal_slot', 'meal_plan_entry',
        'meal_template', 'meal_template_cell', 'grocery_list', 'grocery_item',
        'grocery_item_source', 'meals_change', 'meals_settings'
    ]
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO fa_app', t);
        EXECUTE format('CREATE POLICY tenant ON %I USING (household_id = app_household() OR app_is_system()) WITH CHECK (household_id = app_household() OR app_is_system())', t);
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION set_updated_at()', t || '_updated_at', t);
    END LOOP;
END
$$;
