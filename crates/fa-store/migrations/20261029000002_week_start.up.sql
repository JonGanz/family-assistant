ALTER TABLE household
    ADD COLUMN week_start text NOT NULL DEFAULT 'monday'
    CONSTRAINT household_week_start CHECK (week_start IN ('monday', 'sunday'));
