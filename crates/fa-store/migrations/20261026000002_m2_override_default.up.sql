-- Arch Tasks 4.3: the dashboard completion override is on by default for new households.
ALTER TABLE household ALTER COLUMN dashboard_completion_override SET DEFAULT true;
