-- M3.5 onboarding: system-wide settings, setup state, advisory dismissal removed (D-065, D-068, D-070, D-075).

CREATE TABLE instance_setting (
    key         text PRIMARY KEY,
    value       jsonb,
    ciphertext  bytea,
    nonce       bytea,
    wrapped_dek bytea,
    key_version int,
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER instance_setting_updated_at BEFORE UPDATE ON instance_setting
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
GRANT SELECT, INSERT, UPDATE, DELETE ON instance_setting TO fa_app;

ALTER TABLE household
    ADD COLUMN dashboard_pending_approvals boolean NOT NULL DEFAULT true,
    ADD COLUMN setup_completed_at timestamptz;
UPDATE household SET setup_completed_at = now();

ALTER TABLE advisory DROP COLUMN dismissed_at, DROP COLUMN dismissed_by;
