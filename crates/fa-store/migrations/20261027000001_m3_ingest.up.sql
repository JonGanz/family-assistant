-- M3 ingest: mail sources, messages, orgs, rules pipeline, bills (plan/M3.md, arch/Ingest.md).

ALTER TABLE household
    ADD COLUMN currency text NOT NULL DEFAULT 'USD',
    ADD COLUMN gender_terms jsonb NOT NULL DEFAULT '{"female":["girls","girls''","girl''s"],"male":["boys","boys''","boy''s"]}',
    ADD COLUMN disabled_signals text[] NOT NULL DEFAULT '{}',
    ADD COLUMN events_auto_write boolean NOT NULL DEFAULT false,
    ADD COLUMN events_extract_threshold real NOT NULL DEFAULT 0.9 CHECK (events_extract_threshold BETWEEN 0 AND 1),
    ADD COLUMN events_route_threshold real NOT NULL DEFAULT 0.9 CHECK (events_route_threshold BETWEEN 0 AND 1),
    ADD COLUMN bill_lead_days int NOT NULL DEFAULT 3 CHECK (bill_lead_days BETWEEN 0 AND 60),
    ADD COLUMN inbox_task_expiry_days int NOT NULL DEFAULT 14 CHECK (inbox_task_expiry_days BETWEEN 1 AND 365),
    ADD COLUMN inbox_contact_expiry_days int NOT NULL DEFAULT 14 CHECK (inbox_contact_expiry_days BETWEEN 1 AND 365),
    ADD COLUMN mail_content_policy text NOT NULL DEFAULT 'keep_for_learning'
        CHECK (mail_content_policy IN ('purge_after_processing', 'keep_for_learning', 'keep_all')),
    ADD COLUMN mail_examples_per_category int NOT NULL DEFAULT 200 CHECK (mail_examples_per_category >= 0),
    ADD COLUMN mail_max_pending_days int NOT NULL DEFAULT 14 CHECK (mail_max_pending_days >= 1);

ALTER TABLE member
    ADD COLUMN aliases text[] NOT NULL DEFAULT '{}',
    ADD COLUMN gender text CHECK (gender IN ('female', 'male', 'other')),
    ADD COLUMN relevance_tags text[] NOT NULL DEFAULT '{}';

ALTER TABLE notify_pref
    ADD COLUMN approval_mode text NOT NULL DEFAULT 'batched'
        CHECK (approval_mode IN ('immediate', 'batched', 'digest', 'off'));

-- CHECK replacements (new enum values)
ALTER TABLE task_list DROP CONSTRAINT task_list_kind_check;
ALTER TABLE task_list ADD CONSTRAINT task_list_kind_check CHECK (kind IN ('chores', 'homework', 'general', 'email'));
ALTER TABLE task DROP CONSTRAINT task_origin_check;
ALTER TABLE task ADD CONSTRAINT task_origin_check CHECK (origin IN ('manual', 'system', 'weather', 'email'));
ALTER TABLE notification DROP CONSTRAINT notification_trigger_check;
ALTER TABLE notification ADD CONSTRAINT notification_trigger_check
    CHECK (trigger IN ('assigned', 'overdue', 'digest', 'due_soon', 'routine_nudge', 'source_reauth',
                       'approval_needed', 'mail_time_sensitive'));

CREATE TABLE org (
    id                  uuid PRIMARY KEY,
    household_id        uuid NOT NULL REFERENCES household(id),
    kind                text NOT NULL CHECK (kind IN ('school', 'daycare', 'club', 'biller', 'other')),
    name                text NOT NULL,
    domains             text[] NOT NULL DEFAULT '{}',
    senders             text[] NOT NULL DEFAULT '{}',
    ics_calendar_id     uuid REFERENCES calendar(id) ON DELETE SET NULL,
    default_calendar_id uuid REFERENCES calendar(id) ON DELETE SET NULL,
    contact_mode        text NOT NULL DEFAULT 'suggest' CHECK (contact_mode IN ('suggest', 'auto', 'manual')),
    bill_amount_regex   text,
    bill_due_regex      text,
    bill_account_regex  text,
    archived_at         timestamptz,
    version             int NOT NULL DEFAULT 1,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX org_name_uq ON org (household_id, lower(name)) WHERE archived_at IS NULL;

CREATE TABLE org_contact (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    org_id       uuid NOT NULL REFERENCES org(id) ON DELETE CASCADE,
    name         text NOT NULL,
    email        text NOT NULL,
    role         text NOT NULL DEFAULT 'other' CHECK (role IN ('teacher', 'coach', 'admin', 'office', 'other')),
    archived_at  timestamptz,
    version      int NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, email)
);
CREATE INDEX org_contact_household_idx ON org_contact (household_id);
CREATE INDEX org_contact_email_idx ON org_contact (household_id, email);

CREATE TABLE enrollment (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    member_id    uuid NOT NULL REFERENCES member(id),
    org_id       uuid NOT NULL REFERENCES org(id) ON DELETE CASCADE,
    grade        text,
    class_name   text,
    from_date    date,
    until_date   date,
    version      int NOT NULL DEFAULT 1,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX enrollment_household_idx ON enrollment (household_id);
CREATE INDEX enrollment_member_idx ON enrollment (member_id);
CREATE INDEX enrollment_org_idx ON enrollment (org_id);

CREATE TABLE enrollment_contact (
    enrollment_id uuid NOT NULL REFERENCES enrollment(id) ON DELETE CASCADE,
    contact_id    uuid NOT NULL REFERENCES org_contact(id) ON DELETE CASCADE,
    household_id  uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (enrollment_id, contact_id)
);
CREATE INDEX enrollment_contact_household_idx ON enrollment_contact (household_id);
CREATE INDEX enrollment_contact_contact_idx ON enrollment_contact (contact_id);

CREATE TABLE mail_source (
    id               uuid PRIMARY KEY,
    household_id     uuid NOT NULL REFERENCES household(id),
    provider         text NOT NULL CHECK (provider IN ('gmail', 'imap')),
    name             text NOT NULL,
    address          text NOT NULL,
    oauth_account_id uuid REFERENCES oauth_account(id),
    imap_host        text,
    imap_port        int,
    imap_username    text,
    secret_id        uuid REFERENCES secret(id) ON DELETE SET NULL,
    folders          text[] NOT NULL DEFAULT '{INBOX}',
    archive_folder   text,
    gmail_ext        boolean NOT NULL DEFAULT false,
    read_only        boolean NOT NULL DEFAULT true,
    visibility       text NOT NULL DEFAULT 'owners' CHECK (visibility IN ('owners', 'household')),
    backfill_days    int NOT NULL DEFAULT 30 CHECK (backfill_days BETWEEN 1 AND 365),
    query            text,
    poll_minutes     int NOT NULL DEFAULT 5 CHECK (poll_minutes BETWEEN 1 AND 1440),
    history_id       text,
    state            text NOT NULL DEFAULT 'ok' CHECK (state IN ('ok', 'needs_reauth', 'error', 'paused')),
    last_synced_at   timestamptz,
    last_error       text,
    archived_at      timestamptz,
    version          int NOT NULL DEFAULT 1,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT mail_source_gmail_account CHECK (provider <> 'gmail' OR oauth_account_id IS NOT NULL),
    CONSTRAINT mail_source_imap_login CHECK (provider <> 'imap' OR (imap_host IS NOT NULL AND imap_username IS NOT NULL))
);
CREATE INDEX mail_source_household_idx ON mail_source (household_id);

CREATE TABLE mail_source_owner (
    source_id    uuid NOT NULL REFERENCES mail_source(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    household_id uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (source_id, member_id)
);
CREATE INDEX mail_source_owner_household_idx ON mail_source_owner (household_id);
CREATE INDEX mail_source_owner_member_idx ON mail_source_owner (member_id);

CREATE TABLE mail_folder_state (
    source_id     uuid NOT NULL REFERENCES mail_source(id) ON DELETE CASCADE,
    folder        text NOT NULL,
    household_id  uuid NOT NULL REFERENCES household(id),
    uidvalidity   bigint NOT NULL,
    uidnext       bigint NOT NULL,
    highestmodseq bigint,
    updated_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (source_id, folder)
);
CREATE INDEX mail_folder_state_household_idx ON mail_folder_state (household_id);

CREATE TABLE message (
    id                  uuid PRIMARY KEY,
    household_id        uuid NOT NULL REFERENCES household(id),
    source_id           uuid NOT NULL REFERENCES mail_source(id) ON DELETE CASCADE,
    remote_id           text NOT NULL,
    folder              text,
    thread_id           text,
    rfc822_id           text,
    received_at         timestamptz NOT NULL,
    from_addr           text NOT NULL,
    from_name           text,
    to_addrs            text[] NOT NULL DEFAULT '{}',
    cc_addrs            text[] NOT NULL DEFAULT '{}',
    subject             text NOT NULL DEFAULT '',
    gmail_labels        text[] NOT NULL DEFAULT '{}',
    has_attachments     boolean NOT NULL DEFAULT false,
    category            text,
    time_sensitive      boolean,
    is_bill             boolean,
    processing_state    text NOT NULL DEFAULT 'pending' CHECK (processing_state IN ('pending', 'processed', 'expired')),
    processed_at        timestamptz,
    body_text           text,
    body_html           text,
    snippet             text,
    headers             jsonb,
    attachments         jsonb,
    is_learning_example boolean NOT NULL DEFAULT false,
    in_eval_sample      boolean NOT NULL DEFAULT false,
    content_purged_at   timestamptz,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    UNIQUE (source_id, remote_id)
);
CREATE INDEX message_received_idx ON message (household_id, received_at DESC);
CREATE INDEX message_state_idx ON message (processing_state, received_at);
CREATE INDEX message_category_idx ON message (household_id, category);

CREATE TABLE message_member (
    message_id   uuid NOT NULL REFERENCES message(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    household_id uuid NOT NULL REFERENCES household(id),
    decided_by   text NOT NULL CHECK (decided_by IN ('rule', 'user', 'default')),
    confidence   real NOT NULL DEFAULT 1,
    PRIMARY KEY (message_id, member_id)
);
CREATE INDEX message_member_household_idx ON message_member (household_id);
CREATE INDEX message_member_member_idx ON message_member (member_id);

CREATE TABLE contact_suggestion (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    org_id       uuid NOT NULL REFERENCES org(id) ON DELETE CASCADE,
    email        text NOT NULL,
    name         text,
    member_id    uuid REFERENCES member(id),
    message_id   uuid REFERENCES message(id) ON DELETE SET NULL,
    status       text NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed', 'accepted', 'rejected', 'expired')),
    decided_by   uuid REFERENCES member(id),
    decided_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, email)
);
CREATE INDEX contact_suggestion_household_idx ON contact_suggestion (household_id, status);

CREATE TABLE decision_point (
    id            uuid PRIMARY KEY,
    household_id  uuid NOT NULL REFERENCES household(id),
    key           text NOT NULL CHECK (key IN ('email.category', 'email.time_sensitive', 'email.member_link',
                                               'email.is_bill', 'event.calendar_route')),
    default_value jsonb NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, key)
);

CREATE TABLE pipeline_version (
    id                uuid PRIMARY KEY,
    household_id      uuid NOT NULL REFERENCES household(id),
    decision_point_id uuid NOT NULL REFERENCES decision_point(id) ON DELETE CASCADE,
    version           int NOT NULL,
    active            boolean NOT NULL DEFAULT false,
    name              text NOT NULL,
    created_by        uuid REFERENCES member(id),
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    UNIQUE (decision_point_id, version)
);
CREATE UNIQUE INDEX pipeline_version_active_uq ON pipeline_version (decision_point_id) WHERE active;
CREATE INDEX pipeline_version_household_idx ON pipeline_version (household_id);

CREATE TABLE pipeline_stage (
    id                  uuid PRIMARY KEY,
    household_id        uuid NOT NULL REFERENCES household(id),
    pipeline_version_id uuid NOT NULL REFERENCES pipeline_version(id) ON DELETE CASCADE,
    ord                 int NOT NULL,
    kind                text NOT NULL CHECK (kind IN ('rules', 'human', 'knn', 'system_one', 'llm')),
    enabled             boolean NOT NULL DEFAULT true,
    accept_threshold    real NOT NULL CHECK (accept_threshold BETWEEN 0 AND 1),
    on_unavailable      text NOT NULL DEFAULT 'skip' CHECK (on_unavailable IN ('skip', 'wait', 'queue')),
    config              jsonb NOT NULL DEFAULT '{}',
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    UNIQUE (pipeline_version_id, ord)
);
CREATE INDEX pipeline_stage_household_idx ON pipeline_stage (household_id);

-- Append-only for the app role (no UPDATE); removed by subject in the retention job (no FK).
CREATE TABLE decision (
    id                  uuid PRIMARY KEY,
    household_id        uuid NOT NULL REFERENCES household(id),
    decision_point_id   uuid NOT NULL REFERENCES decision_point(id) ON DELETE CASCADE,
    subject_type        text NOT NULL CHECK (subject_type IN ('message', 'event')),
    subject_id          uuid NOT NULL,
    pipeline_version_id uuid REFERENCES pipeline_version(id) ON DELETE SET NULL,
    stage_kind          text NOT NULL,
    value               jsonb NOT NULL,
    confidence          real NOT NULL,
    is_final            boolean NOT NULL,
    source              text,
    decided_by          uuid REFERENCES member(id),
    created_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX decision_subject_idx ON decision (subject_type, subject_id, decision_point_id, created_at DESC);
CREATE INDEX decision_household_idx ON decision (household_id);

CREATE TABLE mail_rule (
    id             uuid PRIMARY KEY,
    household_id   uuid NOT NULL REFERENCES household(id),
    decision_point text NOT NULL CHECK (decision_point IN ('email.category', 'email.time_sensitive', 'email.member_link',
                                                          'email.is_bill', 'event.calendar_route')),
    name           text NOT NULL,
    priority       int NOT NULL DEFAULT 100,
    conditions     jsonb NOT NULL DEFAULT '[]',
    value          jsonb NOT NULL,
    confidence     real NOT NULL DEFAULT 0.9 CHECK (confidence BETWEEN 0 AND 1),
    enabled        boolean NOT NULL DEFAULT true,
    archived_at    timestamptz,
    version        int NOT NULL DEFAULT 1,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX mail_rule_household_idx ON mail_rule (household_id, decision_point, priority);

CREATE TABLE category (
    id                    uuid PRIMARY KEY,
    household_id          uuid NOT NULL REFERENCES household(id),
    key                   text NOT NULL CHECK (key ~ '^[a-z][a-z0-9_]{0,31}$'),
    label                 text NOT NULL,
    behaviour             text NOT NULL CHECK (behaviour IN ('ignore', 'keep', 'extract')),
    notify_time_sensitive boolean NOT NULL DEFAULT false,
    auto_actions          text[] NOT NULL DEFAULT '{label}',
    email_tasks           text NOT NULL DEFAULT 'suggest' CHECK (email_tasks IN ('suggest', 'create', 'off')),
    color                 text,
    ord                   int NOT NULL DEFAULT 0,
    archived_at           timestamptz,
    version               int NOT NULL DEFAULT 1,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT category_auto_actions CHECK (auto_actions <@ ARRAY['label', 'archive', 'mark_read']),
    UNIQUE (household_id, key)
);

CREATE TABLE bill (
    id               uuid PRIMARY KEY,
    household_id     uuid NOT NULL REFERENCES household(id),
    message_id       uuid REFERENCES message(id) ON DELETE SET NULL,
    org_id           uuid REFERENCES org(id) ON DELETE SET NULL,
    payee            text NOT NULL,
    amount_minor     bigint NOT NULL,
    currency         text NOT NULL,
    due_on           date,
    account_hint     text,
    statement_period text,
    status           text NOT NULL CHECK (status IN ('proposed', 'open', 'paid', 'dismissed')),
    confidence       real NOT NULL,
    task_id          uuid REFERENCES task(id) ON DELETE SET NULL,
    source_ref       jsonb NOT NULL DEFAULT '{}',
    dedupe_key       text NOT NULL,
    version          int NOT NULL DEFAULT 1,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, dedupe_key)
);
CREATE INDEX bill_status_idx ON bill (household_id, status);
CREATE INDEX bill_task_idx ON bill (task_id);
CREATE INDEX bill_message_idx ON bill (message_id);

CREATE TABLE mail_action (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    message_id   uuid NOT NULL REFERENCES message(id) ON DELETE CASCADE,
    action       text NOT NULL CHECK (action IN ('archive', 'mark_read', 'label')),
    label        text,
    automatic    boolean NOT NULL DEFAULT false,
    done_by      uuid REFERENCES member(id),
    done_at      timestamptz NOT NULL DEFAULT now(),
    undone_at    timestamptz,
    undone_by    uuid REFERENCES member(id),
    error        text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX mail_action_message_idx ON mail_action (message_id);
CREATE INDEX mail_action_household_idx ON mail_action (household_id);

-- event
ALTER TABLE event DROP CONSTRAINT event_status_check;
ALTER TABLE event ADD CONSTRAINT event_status_check
    CHECK (status IN ('extracted', 'proposed', 'approved', 'written', 'rejected', 'expired'));
ALTER TABLE event
    ADD COLUMN message_id         uuid REFERENCES message(id) ON DELETE SET NULL,
    ADD COLUMN org_id             uuid REFERENCES org(id) ON DELETE SET NULL,
    ADD COLUMN target_calendar_id uuid REFERENCES calendar(id) ON DELETE SET NULL,
    ADD COLUMN route_confidence   real,
    ADD COLUMN pending_cancel     boolean NOT NULL DEFAULT false,
    ADD COLUMN write_error        text,
    ADD COLUMN decided_by         uuid REFERENCES member(id),
    ADD COLUMN decided_at         timestamptz;
CREATE INDEX event_extracted_status_idx ON event (household_id, status) WHERE origin = 'extracted';
CREATE INDEX event_message_idx ON event (message_id);

CREATE TABLE event_message (
    event_id     uuid NOT NULL REFERENCES event(id) ON DELETE CASCADE,
    message_id   uuid NOT NULL REFERENCES message(id) ON DELETE CASCADE,
    household_id uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (event_id, message_id)
);
CREATE INDEX event_message_household_idx ON event_message (household_id);
CREATE INDEX event_message_message_idx ON event_message (message_id);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['org','org_contact','enrollment','enrollment_contact','mail_source','mail_source_owner',
                             'mail_folder_state','message','message_member','contact_suggestion','decision_point',
                             'pipeline_version','pipeline_stage','decision','mail_rule','category','bill',
                             'mail_action','event_message']
    LOOP
        IF t NOT IN ('enrollment_contact','mail_source_owner','message_member','decision','event_message') THEN
            EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION set_updated_at()', t || '_updated_at', t);
        END IF;
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        IF t = 'decision' THEN
            EXECUTE format('GRANT SELECT, INSERT, DELETE ON %I TO fa_app', t);
        ELSE
            EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO fa_app', t);
        END IF;
        EXECUTE format('CREATE POLICY tenant ON %I USING (household_id = app_household() OR app_is_system()) WITH CHECK (household_id = app_household() OR app_is_system())', t);
    END LOOP;
END
$$;

-- Seed the D-054 categories and the five decision points ('Rules only' pipeline) for existing households.
SELECT set_config('app.system', 'on', true);

INSERT INTO category (id, household_id, key, label, behaviour, notify_time_sensitive, ord)
SELECT gen_random_uuid(), h.id, c.key, c.label, c.behaviour, c.notify, c.ord
FROM household h
CROSS JOIN (VALUES
    ('school', 'School', 'extract', true, 0),
    ('daycare', 'Daycare', 'extract', true, 1),
    ('activities', 'Activities', 'extract', true, 2),
    ('appointments', 'Appointments', 'extract', true, 3),
    ('bills', 'Bills', 'extract', false, 4),
    ('household', 'Household', 'extract', false, 5),
    ('personal', 'Personal', 'keep', false, 6),
    ('newsletters', 'Newsletters', 'keep', false, 7),
    ('promotions', 'Promotions', 'ignore', false, 8),
    ('social', 'Social', 'ignore', false, 9),
    ('junk', 'Junk', 'ignore', false, 10),
    ('other', 'Other', 'keep', false, 11)
) AS c(key, label, behaviour, notify, ord)
ON CONFLICT (household_id, key) DO NOTHING;

INSERT INTO decision_point (id, household_id, key, default_value)
SELECT gen_random_uuid(), h.id, d.key, d.val::jsonb
FROM household h
CROSS JOIN (VALUES
    ('email.category', '"other"'),
    ('email.time_sensitive', 'false'),
    ('email.member_link', '"owners"'),
    ('email.is_bill', 'false'),
    ('event.calendar_route', '"propose"')
) AS d(key, val)
ON CONFLICT (household_id, key) DO NOTHING;

INSERT INTO pipeline_version (id, household_id, decision_point_id, version, active, name)
SELECT gen_random_uuid(), dp.household_id, dp.id, 1, true, 'Rules only' FROM decision_point dp
ON CONFLICT (decision_point_id, version) DO NOTHING;

INSERT INTO pipeline_stage (id, household_id, pipeline_version_id, ord, kind, accept_threshold)
SELECT gen_random_uuid(), v.household_id, v.id, s.ord, s.kind, s.thr
FROM pipeline_version v
CROSS JOIN (VALUES (0, 'rules', 0.8::real), (1, 'human', 0.0::real)) AS s(ord, kind, thr)
ON CONFLICT (pipeline_version_id, ord) DO NOTHING;
