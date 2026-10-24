CREATE TABLE household (
    id                            uuid PRIMARY KEY,
    name                          text NOT NULL,
    tz                            text NOT NULL,
    location_lat                  double precision,
    location_lon                  double precision,
    location_label                text,
    meal_plan_horizon_weeks       int NOT NULL DEFAULT 2 CHECK (meal_plan_horizon_weeks > 0),
    dashboard_completion_override boolean NOT NULL DEFAULT false,
    version                       int NOT NULL DEFAULT 1,
    created_at                    timestamptz NOT NULL DEFAULT now(),
    updated_at                    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT household_location_all_or_none CHECK (
        (location_lat IS NULL AND location_lon IS NULL AND location_label IS NULL)
        OR (location_lat IS NOT NULL AND location_lon IS NOT NULL AND location_label IS NOT NULL))
);

CREATE TABLE member (
    id                 uuid PRIMARY KEY,
    household_id       uuid NOT NULL REFERENCES household(id),
    config_key         text,
    display_name       text NOT NULL,
    role               text NOT NULL CHECK (role IN ('adult', 'child')),
    avatar             text,
    color              text,
    birthdate          date,
    grade              text,
    interests          text[] NOT NULL DEFAULT '{}',
    hobbies            text[] NOT NULL DEFAULT '{}',
    responsibilities   text[] NOT NULL DEFAULT '{}',
    pin_hash           text,
    pin_failed_attempts int NOT NULL DEFAULT 0,
    pin_locked_until   timestamptz,
    archived_at        timestamptz,
    version            int NOT NULL DEFAULT 1,
    created_by         uuid REFERENCES member(id),
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT member_pin_adults_only CHECK (pin_hash IS NULL OR role = 'adult')
);
CREATE UNIQUE INDEX member_config_key_uq ON member (household_id, config_key) WHERE config_key IS NOT NULL;
CREATE INDEX member_household_idx ON member (household_id);

CREATE TABLE user_account (
    id            uuid PRIMARY KEY,
    household_id  uuid NOT NULL REFERENCES household(id),
    member_id     uuid NOT NULL UNIQUE REFERENCES member(id),
    username      text NOT NULL,
    password_hash text NOT NULL,
    failed_attempts int NOT NULL DEFAULT 0,
    locked_until  timestamptz,
    disabled_at   timestamptz,
    version       int NOT NULL DEFAULT 1,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX user_account_username_uq ON user_account (household_id, lower(username));

-- ADR: own session table + custom tower-sessions store (implemented in fa-auth),
-- not tower-sessions-sqlx-store's table. The stock table (id, data, expiry_date) has no
-- household or member columns, so RLS could not apply and "adults revoke a child's
-- sessions" (D-031) would need a scan of serialized blobs.
CREATE TABLE session (
    id           text PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    member_id    uuid NOT NULL REFERENCES member(id),
    account_id   uuid NOT NULL REFERENCES user_account(id) ON DELETE CASCADE,
    data         bytea NOT NULL,
    user_agent   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen    timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL,
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX session_member_idx ON session (member_id);
CREATE INDEX session_expires_idx ON session (expires_at);

CREATE TABLE device (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    kind         text NOT NULL CHECK (kind IN ('dashboard')),
    name         text NOT NULL,
    token_hash   text NOT NULL UNIQUE,
    last_seen    timestamptz,
    revoked_at   timestamptz,
    version      int NOT NULL DEFAULT 1,
    created_by   uuid REFERENCES member(id),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX device_household_idx ON device (household_id);

CREATE TABLE device_elevation (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    household_id uuid NOT NULL REFERENCES household(id),
    device_id    uuid NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    expires_at   timestamptz NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX device_elevation_device_idx ON device_elevation (device_id, expires_at);

CREATE TABLE notify_pref (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    member_id    uuid NOT NULL REFERENCES member(id),
    channel      text NOT NULL CHECK (channel IN ('ntfy')),
    topic        text NOT NULL,
    triggers     text[] NOT NULL DEFAULT '{}',
    archived_at  timestamptz,
    version      int NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX notify_pref_member_idx ON notify_pref (member_id);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['household','member','user_account','session','device','device_elevation','notify_pref']
    LOOP
        EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION set_updated_at()', t || '_updated_at', t);
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO fa_app', t);
        IF t = 'household' THEN
            EXECUTE 'CREATE POLICY tenant ON household USING (id = app_household() OR app_is_system()) WITH CHECK (id = app_household() OR app_is_system())';
        ELSE
            EXECUTE format('CREATE POLICY tenant ON %I USING (household_id = app_household() OR app_is_system()) WITH CHECK (household_id = app_household() OR app_is_system())', t);
        END IF;
    END LOOP;
END
$$;
