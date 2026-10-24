CREATE TABLE audit_entry (
    id              uuid PRIMARY KEY,
    household_id    uuid NOT NULL REFERENCES household(id),
    actor_member_id uuid REFERENCES member(id),
    device_id       uuid REFERENCES device(id),
    action          text NOT NULL,
    subject_type    text NOT NULL,
    subject_id      text,
    before          jsonb,
    after           jsonb,
    at              timestamptz NOT NULL DEFAULT now(),
    created_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_entry_household_at_idx ON audit_entry (household_id, at DESC);

-- Append-only: the app role has no UPDATE/DELETE/TRUNCATE, and triggers also stop the owner.
CREATE FUNCTION audit_entry_immutable() RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    RAISE EXCEPTION 'audit_entry is append-only' USING ERRCODE = 'insufficient_privilege';
END
$$;
CREATE TRIGGER audit_entry_no_change BEFORE UPDATE OR DELETE ON audit_entry
    FOR EACH ROW EXECUTE FUNCTION audit_entry_immutable();
CREATE TRIGGER audit_entry_no_truncate BEFORE TRUNCATE ON audit_entry
    FOR EACH STATEMENT EXECUTE FUNCTION audit_entry_immutable();
ALTER TABLE audit_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_entry FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant ON audit_entry
    USING (household_id = app_household() OR app_is_system())
    WITH CHECK (household_id = app_household() OR app_is_system());
GRANT SELECT, INSERT ON audit_entry TO fa_app;

CREATE TABLE secret (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    owner_type   text NOT NULL,
    owner_id     uuid NOT NULL,
    kind         text NOT NULL CHECK (kind IN ('oauth_token', 'other')),
    ciphertext   bytea NOT NULL,
    nonce        bytea NOT NULL,
    wrapped_dek  bytea NOT NULL,
    key_version  int NOT NULL,
    version      int NOT NULL DEFAULT 1,
    created_by   uuid REFERENCES member(id),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, owner_type, owner_id, kind)
);
CREATE TRIGGER secret_updated_at BEFORE UPDATE ON secret FOR EACH ROW EXECUTE FUNCTION set_updated_at();
ALTER TABLE secret ENABLE ROW LEVEL SECURITY;
ALTER TABLE secret FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant ON secret
    USING (household_id = app_household() OR app_is_system())
    WITH CHECK (household_id = app_household() OR app_is_system());
GRANT SELECT, INSERT, UPDATE, DELETE ON secret TO fa_app;

-- System-wide queue tables (no RLS): workers claim across households. household_id is informational.
CREATE TABLE job (
    id              uuid PRIMARY KEY,
    household_id    uuid REFERENCES household(id),
    kind            text NOT NULL,
    payload         jsonb NOT NULL DEFAULT '{}',
    status          text NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'done', 'dead')),
    run_at          timestamptz NOT NULL DEFAULT now(),
    priority        int NOT NULL DEFAULT 0,
    attempts        int NOT NULL DEFAULT 0,
    max_attempts    int NOT NULL DEFAULT 5,
    locked_by       text,
    locked_until    timestamptz,
    idempotency_key text UNIQUE,
    last_error      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX job_claim_idx ON job (priority DESC, run_at) WHERE status = 'queued';
CREATE TRIGGER job_updated_at BEFORE UPDATE ON job FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE schedule (
    id               uuid PRIMARY KEY,
    name             text NOT NULL UNIQUE,
    cron             text NOT NULL,
    job_kind         text NOT NULL,
    payload          jsonb NOT NULL DEFAULT '{}',
    enabled          boolean NOT NULL DEFAULT true,
    last_enqueued_at timestamptz,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER schedule_updated_at BEFORE UPDATE ON schedule FOR EACH ROW EXECUTE FUNCTION set_updated_at();
GRANT SELECT, INSERT, UPDATE, DELETE ON job, schedule TO fa_app;
