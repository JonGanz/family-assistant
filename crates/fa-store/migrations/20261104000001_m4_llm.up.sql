-- M4: LLM endpoint, AI settings, call metrics, and extraction provenance (plan/M4.md, arch/AI.md).

ALTER TABLE secret DROP CONSTRAINT secret_kind_check;
ALTER TABLE secret ADD CONSTRAINT secret_kind_check CHECK (kind IN ('oauth_token', 'other', 'api_key'));

CREATE TABLE llm_endpoint (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL UNIQUE REFERENCES household(id),
    base_url     text NOT NULL,
    model        text NOT NULL,
    dialect      text NOT NULL DEFAULT 'openai_chat' CHECK (dialect IN ('openai_chat', 'ollama_native')),
    timeout_ms   int NOT NULL DEFAULT 120000 CHECK (timeout_ms BETWEEN 1000 AND 600000),
    enabled      boolean NOT NULL DEFAULT false,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER llm_endpoint_updated_at BEFORE UPDATE ON llm_endpoint FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE ai_settings (
    household_id      uuid PRIMARY KEY REFERENCES household(id),
    mail_extract_mode text NOT NULL DEFAULT 'fallback' CHECK (mail_extract_mode IN ('off', 'fallback', 'always')),
    max_input_chars   int NOT NULL DEFAULT 12000 CHECK (max_input_chars BETWEEN 1000 AND 100000),
    updated_at        timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER ai_settings_updated_at BEFORE UPDATE ON ai_settings FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE llm_call (
    id                uuid PRIMARY KEY,
    household_id      uuid NOT NULL REFERENCES household(id),
    purpose           text NOT NULL,
    model             text NOT NULL,
    started_at        timestamptz NOT NULL,
    latency_ms        int NOT NULL,
    ok                boolean NOT NULL,
    error_kind        text,
    prompt_tokens     int,
    completion_tokens int
);
CREATE INDEX llm_call_household_started_idx ON llm_call (household_id, started_at DESC);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['llm_endpoint', 'ai_settings', 'llm_call']
    LOOP
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO fa_app', t);
        EXECUTE format('CREATE POLICY tenant ON %I USING (household_id = app_household() OR app_is_system()) WITH CHECK (household_id = app_household() OR app_is_system())', t);
    END LOOP;
END
$$;

ALTER TABLE event ADD COLUMN extracted_by text NOT NULL DEFAULT 'rules' CHECK (extracted_by IN ('rules', 'llm'));
ALTER TABLE bill ADD COLUMN extracted_by text NOT NULL DEFAULT 'rules' CHECK (extracted_by IN ('rules', 'llm'));
ALTER TABLE task ADD COLUMN extracted_by text NOT NULL DEFAULT 'rules' CHECK (extracted_by IN ('rules', 'llm'));
