-- M1 feeds: calendar read, weather, OAuth accounts (arch/Feeds.md section 2).

ALTER TABLE household
    ADD COLUMN units text NOT NULL DEFAULT 'metric' CHECK (units IN ('metric', 'imperial')),
    ADD COLUMN calendar_past_days int NOT NULL DEFAULT 30 CHECK (calendar_past_days BETWEEN 0 AND 3650),
    ADD COLUMN calendar_future_days int NOT NULL DEFAULT 400 CHECK (calendar_future_days BETWEEN 1 AND 3650),
    ADD COLUMN weather_retention_days int NOT NULL DEFAULT 7 CHECK (weather_retention_days BETWEEN 1 AND 365),
    ADD COLUMN private_events_as_busy boolean NOT NULL DEFAULT true;

ALTER TABLE device ADD COLUMN settings jsonb NOT NULL DEFAULT '{}';

CREATE TABLE member_avatar (
    member_id    uuid PRIMARY KEY REFERENCES member(id) ON DELETE CASCADE,
    household_id uuid NOT NULL REFERENCES household(id),
    mime         text NOT NULL CHECK (mime IN ('image/png', 'image/jpeg', 'image/webp')),
    bytes        bytea NOT NULL CHECK (octet_length(bytes) <= 524288),
    sha256       text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX member_avatar_household_idx ON member_avatar (household_id);

CREATE TABLE oauth_account (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    provider     text NOT NULL CHECK (provider IN ('google')),
    owner_member uuid NOT NULL REFERENCES member(id),
    email        text NOT NULL,
    scopes       text[] NOT NULL DEFAULT '{}',
    secret_id    uuid REFERENCES secret(id) ON DELETE SET NULL,
    state        text NOT NULL DEFAULT 'ok' CHECK (state IN ('ok', 'needs_reauth', 'error')),
    last_error   text,
    version      int NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, provider, email)
);
CREATE INDEX oauth_account_owner_idx ON oauth_account (owner_member);

CREATE TABLE oauth_pending (
    state         text PRIMARY KEY,
    household_id  uuid NOT NULL REFERENCES household(id),
    session_id    text NOT NULL,
    member_id     uuid NOT NULL REFERENCES member(id),
    pkce_verifier text NOT NULL,
    return_to     text,
    expires_at    timestamptz NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX oauth_pending_expires_idx ON oauth_pending (expires_at);

CREATE TABLE calendar_source (
    id             uuid PRIMARY KEY,
    household_id   uuid NOT NULL REFERENCES household(id),
    kind           text NOT NULL CHECK (kind IN ('google', 'caldav', 'ics')),
    name           text NOT NULL,
    owner_member   uuid REFERENCES member(id),
    oauth_account  uuid REFERENCES oauth_account(id),
    url            text,
    username       text,
    secret_id      uuid REFERENCES secret(id) ON DELETE SET NULL,
    writable       boolean NOT NULL DEFAULT false,
    enabled        boolean NOT NULL DEFAULT true,
    poll_minutes   int NOT NULL DEFAULT 15 CHECK (poll_minutes BETWEEN 1 AND 10080),
    state          text NOT NULL DEFAULT 'ok' CHECK (state IN ('ok', 'needs_reauth', 'error')),
    last_synced_at timestamptz,
    last_error     text,
    archived_at    timestamptz,
    version        int NOT NULL DEFAULT 1,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX calendar_source_household_idx ON calendar_source (household_id);

CREATE TABLE calendar (
    id                uuid PRIMARY KEY,
    household_id      uuid NOT NULL REFERENCES household(id),
    source_id         uuid NOT NULL REFERENCES calendar_source(id) ON DELETE CASCADE,
    remote_id         text NOT NULL,
    name              text NOT NULL,
    color             text,
    enabled           boolean NOT NULL DEFAULT false,
    is_default_target boolean NOT NULL DEFAULT false,
    sync_state        text,
    last_full_sync_at timestamptz,
    archived_at       timestamptz,
    version           int NOT NULL DEFAULT 1,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    UNIQUE (source_id, remote_id)
);
CREATE INDEX calendar_household_idx ON calendar (household_id);

CREATE TABLE calendar_member (
    calendar_id  uuid NOT NULL REFERENCES calendar(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    household_id uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (calendar_id, member_id)
);
CREATE INDEX calendar_member_household_idx ON calendar_member (household_id);

CREATE TABLE event (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    calendar_id  uuid REFERENCES calendar(id) ON DELETE CASCADE,
    remote_uid   text,
    instance_key text NOT NULL DEFAULT '',
    title        text NOT NULL,
    description  text,
    location     text,
    start_at     timestamptz NOT NULL,
    end_at       timestamptz NOT NULL,
    all_day      boolean NOT NULL DEFAULT false,
    start_date   date,
    end_date     date,
    source_tz    text,
    is_private   boolean NOT NULL DEFAULT false,
    cancelled    boolean NOT NULL DEFAULT false,
    origin       text NOT NULL CHECK (origin IN ('synced', 'extracted', 'manual')),
    status       text CHECK (status IN ('proposed', 'approved', 'written', 'rejected')),
    confidence   real,
    source_ref   jsonb,
    dedupe_key   text NOT NULL,
    remote_etag  text,
    version      int NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT event_origin_status CHECK ((origin = 'synced') = (status IS NULL)),
    CONSTRAINT event_end_after_start CHECK (end_at >= start_at),
    UNIQUE (calendar_id, remote_uid, instance_key)
);
CREATE INDEX event_household_start_idx ON event (household_id, start_at);
CREATE INDEX event_calendar_start_idx ON event (calendar_id, start_at);

CREATE TABLE event_member (
    event_id     uuid NOT NULL REFERENCES event(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    household_id uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (event_id, member_id)
);
CREATE INDEX event_member_household_idx ON event_member (household_id);

CREATE TABLE weather_snapshot (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    fetched_at   timestamptz NOT NULL DEFAULT now(),
    provider     text NOT NULL,
    "current"    jsonb NOT NULL,
    hourly       jsonb NOT NULL,
    daily        jsonb NOT NULL
);
CREATE INDEX weather_snapshot_fetched_idx ON weather_snapshot (household_id, fetched_at DESC);

CREATE TABLE weather_rule (
    id              uuid PRIMARY KEY,
    household_id    uuid NOT NULL REFERENCES household(id),
    name            text NOT NULL,
    condition       text NOT NULL,
    action          text NOT NULL CHECK (action IN ('alert', 'task')),
    severity        text NOT NULL CHECK (severity IN ('info', 'warning', 'severe')),
    message         text NOT NULL,
    task_definition jsonb,
    status          text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suggested', 'disabled')),
    suggested_by    text NOT NULL DEFAULT 'user' CHECK (suggested_by IN ('default', 's2', 'user')),
    archived_at     timestamptz,
    version         int NOT NULL DEFAULT 1,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX weather_rule_household_idx ON weather_rule (household_id);

CREATE TABLE advisory (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    rule_id      uuid REFERENCES weather_rule(id) ON DELETE SET NULL,
    text         text NOT NULL,
    severity     text NOT NULL CHECK (severity IN ('info', 'warning', 'severe')),
    valid_from   timestamptz NOT NULL,
    valid_until  timestamptz NOT NULL,
    source       text NOT NULL CHECK (source IN ('rule', 's2')),
    dedupe_key   text NOT NULL,
    dismissed_at timestamptz,
    dismissed_by uuid REFERENCES member(id),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, dedupe_key)
);
CREATE INDEX advisory_active_idx ON advisory (household_id, valid_until);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['member_avatar','oauth_account','oauth_pending','calendar_source','calendar',
                             'calendar_member','event','event_member','weather_snapshot','weather_rule','advisory']
    LOOP
        IF t NOT IN ('calendar_member','event_member','weather_snapshot') THEN
            EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION set_updated_at()', t || '_updated_at', t);
        END IF;
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO fa_app', t);
        EXECUTE format('CREATE POLICY tenant ON %I USING (household_id = app_household() OR app_is_system()) WITH CHECK (household_id = app_household() OR app_is_system())', t);
    END LOOP;
END
$$;
