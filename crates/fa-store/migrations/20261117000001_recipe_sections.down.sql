ALTER TABLE meals_settings
    DROP COLUMN recipe_scale_factors,
    DROP COLUMN recipe_section_columns;
ALTER TABLE recipe_step DROP COLUMN section_id;
ALTER TABLE recipe_ingredient DROP COLUMN section_id;
DROP TABLE recipe_section;
