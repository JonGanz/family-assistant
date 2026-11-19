CREATE TABLE recipe_section (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    recipe_id    uuid NOT NULL REFERENCES recipe(id) ON DELETE CASCADE,
    ord          int NOT NULL,
    name         text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (recipe_id, ord)
);

ALTER TABLE recipe_section ENABLE ROW LEVEL SECURITY;
ALTER TABLE recipe_section FORCE ROW LEVEL SECURITY;
GRANT SELECT, INSERT, UPDATE, DELETE ON recipe_section TO fa_app;
CREATE POLICY tenant ON recipe_section
    USING (household_id = app_household() OR app_is_system())
    WITH CHECK (household_id = app_household() OR app_is_system());
CREATE TRIGGER recipe_section_updated_at BEFORE UPDATE ON recipe_section
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE recipe_ingredient ADD COLUMN section_id uuid REFERENCES recipe_section(id) ON DELETE SET NULL;
ALTER TABLE recipe_step ADD COLUMN section_id uuid REFERENCES recipe_section(id) ON DELETE SET NULL;
CREATE INDEX recipe_ingredient_section_idx ON recipe_ingredient (section_id);
CREATE INDEX recipe_step_section_idx ON recipe_step (section_id);

ALTER TABLE meals_settings
    ADD COLUMN recipe_scale_factors real[] NOT NULL DEFAULT '{0.5,1,1.5,2,3}',
    ADD COLUMN recipe_section_columns int NOT NULL DEFAULT 2 CHECK (recipe_section_columns BETWEEN 1 AND 4);
