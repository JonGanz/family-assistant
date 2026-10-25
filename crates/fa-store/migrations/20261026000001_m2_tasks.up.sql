-- M2 tasks, routines, notifications (arch/Tasks.md section 3).

ALTER TABLE household
    ADD COLUMN now_list_size int NOT NULL DEFAULT 7 CHECK (now_list_size BETWEEN 1 AND 50),
    ADD COLUMN now_weights jsonb NOT NULL DEFAULT '{"due":1.0,"flag":0.5,"weather":0.5}',
    ADD COLUMN quiet_from time NOT NULL DEFAULT '21:00',
    ADD COLUMN quiet_until time NOT NULL DEFAULT '07:00';

ALTER TABLE weather_rule
    ADD COLUMN task_mode text NOT NULL DEFAULT 'create' CHECK (task_mode IN ('create', 'suggest'));

ALTER TABLE notify_pref
    ADD COLUMN due_soon_minutes int NOT NULL DEFAULT 120 CHECK (due_soon_minutes >= 0),
    ADD COLUMN nudge_minutes int NOT NULL DEFAULT 10 CHECK (nudge_minutes >= 0),
    ADD COLUMN quiet_from time,
    ADD COLUMN quiet_until time,
    ADD COLUMN copy_to_member_id uuid REFERENCES member(id);
ALTER TABLE notify_pref ALTER COLUMN triggers SET DEFAULT '{assigned,overdue,digest}';
CREATE UNIQUE INDEX notify_pref_member_uq ON notify_pref (member_id) WHERE archived_at IS NULL;

CREATE TABLE task_list (
    id                 uuid PRIMARY KEY,
    household_id       uuid NOT NULL REFERENCES household(id),
    name               text NOT NULL,
    kind               text NOT NULL CHECK (kind IN ('chores', 'homework', 'general')),
    completion_policy  text NOT NULL CHECK (completion_policy IN ('assignee', 'adult_only')),
    child_can_add      text NOT NULL CHECK (child_can_add IN ('own', 'none')),
    dashboard_add      boolean NOT NULL DEFAULT false,
    calendar_default   text NOT NULL DEFAULT 'hidden' CHECK (calendar_default IN ('hidden', 'until_done', 'always')),
    claim_ttl_minutes  int CHECK (claim_ttl_minutes IS NULL OR claim_ttl_minutes > 0),
    visible_on_dashboard boolean NOT NULL DEFAULT true,
    color              text,
    sort               int NOT NULL DEFAULT 0,
    archived_at        timestamptz,
    version            int NOT NULL DEFAULT 1,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX task_list_household_idx ON task_list (household_id);

CREATE TABLE task_definition (
    id                   uuid PRIMARY KEY,
    household_id         uuid NOT NULL REFERENCES household(id),
    list_id              uuid NOT NULL REFERENCES task_list(id),
    title                text NOT NULL,
    notes                text,
    steps_template       text[] NOT NULL DEFAULT '{}',
    schedule_kind        text NOT NULL CHECK (schedule_kind IN ('after_done', 'rrule')),
    interval_days        int CHECK (interval_days IS NULL OR interval_days >= 1),
    rrule                text,
    start_date           date NOT NULL,
    due_time             time,
    lead_days            int NOT NULL DEFAULT 0 CHECK (lead_days >= 0),
    on_missed            text NOT NULL CHECK (on_missed IN ('roll_over', 'auto_skip', 'pile_up')),
    strategy             text NOT NULL CHECK (strategy IN ('fixed', 'rotation', 'pool')),
    advance_on           text NOT NULL DEFAULT 'any_completion' CHECK (advance_on IN ('any_completion', 'assignee_completion')),
    skip_advances        boolean NOT NULL DEFAULT true,
    rotation_cursor      int NOT NULL DEFAULT 0,
    priority_flag        text NOT NULL DEFAULT 'normal' CHECK (priority_flag IN ('normal', 'high')),
    season_from          int,
    season_until         int,
    season_condition     text,
    condition_ok         boolean,
    condition_checked_at timestamptz,
    last_done_at         timestamptz,
    next_due_at          timestamptz,
    paused               boolean NOT NULL DEFAULT false,
    archived_at          timestamptz,
    version              int NOT NULL DEFAULT 1,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT task_definition_after_done_interval CHECK ((schedule_kind = 'after_done') = (interval_days IS NOT NULL)),
    CONSTRAINT task_definition_rrule_body CHECK ((schedule_kind = 'rrule') = (rrule IS NOT NULL)),
    CONSTRAINT task_definition_season_both CHECK ((season_from IS NULL) = (season_until IS NULL))
);
CREATE INDEX task_definition_household_idx ON task_definition (household_id);
CREATE INDEX task_definition_list_idx ON task_definition (list_id);

CREATE TABLE task_definition_candidate (
    definition_id uuid NOT NULL REFERENCES task_definition(id) ON DELETE CASCADE,
    member_id     uuid NOT NULL REFERENCES member(id),
    household_id  uuid NOT NULL REFERENCES household(id),
    ord           int NOT NULL,
    PRIMARY KEY (definition_id, member_id),
    UNIQUE (definition_id, ord)
);
CREATE INDEX task_definition_candidate_household_idx ON task_definition_candidate (household_id);

CREATE TABLE task (
    id                  uuid PRIMARY KEY,
    household_id        uuid NOT NULL REFERENCES household(id),
    list_id             uuid NOT NULL REFERENCES task_list(id),
    definition_id       uuid REFERENCES task_definition(id),
    weather_rule_id     uuid REFERENCES weather_rule(id) ON DELETE SET NULL,
    title               text NOT NULL,
    notes               text,
    due_at              timestamptz,
    visible_from        timestamptz,
    status              text NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'done', 'skipped', 'suggested')),
    origin              text NOT NULL CHECK (origin IN ('manual', 'system', 'weather')),
    source_ref          jsonb,
    priority_flag       text NOT NULL DEFAULT 'normal' CHECK (priority_flag IN ('normal', 'high')),
    calendar_display    text NOT NULL DEFAULT 'hidden' CHECK (calendar_display IN ('hidden', 'until_done', 'always')),
    created_by          uuid REFERENCES member(id),
    completed_at        timestamptz,
    completed_by        uuid REFERENCES member(id),
    completed_via_device uuid REFERENCES device(id) ON DELETE SET NULL,
    skipped_by          uuid REFERENCES member(id),
    claimed_by          uuid REFERENCES member(id),
    claimed_until       timestamptz,
    completion_undo     jsonb,
    archived_at         timestamptz,
    version             int NOT NULL DEFAULT 1,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    UNIQUE (definition_id, origin, due_at)
);
CREATE INDEX task_status_due_idx ON task (household_id, status, due_at);
CREATE INDEX task_definition_status_idx ON task (definition_id, status);

CREATE TABLE task_assignee (
    task_id      uuid NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    household_id uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (task_id, member_id)
);
CREATE INDEX task_assignee_member_idx ON task_assignee (member_id);
CREATE INDEX task_assignee_household_idx ON task_assignee (household_id);

CREATE TABLE task_step (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    task_id      uuid NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    ord          int NOT NULL,
    text         text NOT NULL,
    done_at      timestamptz,
    done_by      uuid REFERENCES member(id),
    UNIQUE (task_id, ord)
);
CREATE INDEX task_step_household_idx ON task_step (household_id);

CREATE TABLE member_away (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    member_id    uuid NOT NULL REFERENCES member(id),
    from_date    date NOT NULL,
    until_date   date NOT NULL,
    note         text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT member_away_range CHECK (until_date >= from_date)
);
CREATE INDEX member_away_member_idx ON member_away (member_id, from_date);
CREATE INDEX member_away_household_idx ON member_away (household_id);

CREATE TABLE routine (
    id                  uuid PRIMARY KEY,
    household_id        uuid NOT NULL REFERENCES household(id),
    name                text NOT NULL,
    starts_at           time NOT NULL,
    done_by             time NOT NULL,
    day_filter          text NOT NULL DEFAULT 'weekdays' CHECK (day_filter IN ('daily', 'weekdays', 'weekends', 'school_days')),
    school_calendar_id  uuid REFERENCES calendar(id) ON DELETE SET NULL,
    no_school_match     text[] NOT NULL DEFAULT '{no school,holiday,inset,closed}',
    show_on_dashboard   boolean NOT NULL DEFAULT true,
    archived_at         timestamptz,
    version             int NOT NULL DEFAULT 1,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX routine_household_idx ON routine (household_id);

CREATE TABLE routine_member (
    routine_id   uuid NOT NULL REFERENCES routine(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    household_id uuid NOT NULL REFERENCES household(id),
    PRIMARY KEY (routine_id, member_id)
);
CREATE INDEX routine_member_household_idx ON routine_member (household_id);

CREATE TABLE routine_step (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    routine_id   uuid NOT NULL REFERENCES routine(id) ON DELETE CASCADE,
    ord          int NOT NULL,
    text         text NOT NULL,
    target_time  time,
    UNIQUE (routine_id, ord)
);
CREATE INDEX routine_step_household_idx ON routine_step (household_id);

CREATE TABLE routine_run (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    routine_id   uuid NOT NULL REFERENCES routine(id) ON DELETE CASCADE,
    member_id    uuid NOT NULL REFERENCES member(id),
    run_date     date NOT NULL,
    status       text NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'done', 'skipped')),
    completed_at timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (routine_id, member_id, run_date)
);
CREATE INDEX routine_run_date_idx ON routine_run (household_id, run_date);

CREATE TABLE routine_run_step (
    run_id          uuid NOT NULL REFERENCES routine_run(id) ON DELETE CASCADE,
    step_id         uuid NOT NULL REFERENCES routine_step(id) ON DELETE CASCADE,
    household_id    uuid NOT NULL REFERENCES household(id),
    done_at         timestamptz,
    done_by         uuid REFERENCES member(id),
    done_via_device uuid REFERENCES device(id) ON DELETE SET NULL,
    PRIMARY KEY (run_id, step_id)
);
CREATE INDEX routine_run_step_household_idx ON routine_run_step (household_id);

CREATE TABLE notification (
    id           uuid PRIMARY KEY,
    household_id uuid NOT NULL REFERENCES household(id),
    member_id    uuid NOT NULL REFERENCES member(id),
    trigger      text NOT NULL CHECK (trigger IN ('assigned', 'overdue', 'digest', 'due_soon', 'routine_nudge', 'source_reauth')),
    subject_type text NOT NULL,
    subject_id   uuid,
    title        text NOT NULL,
    body         text NOT NULL DEFAULT '',
    click_path   text,
    priority     int NOT NULL DEFAULT 3 CHECK (priority BETWEEN 1 AND 5),
    dedupe_key   text NOT NULL,
    status       text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'dropped', 'failed')),
    not_before   timestamptz NOT NULL DEFAULT now(),
    sent_at      timestamptz,
    attempts     int NOT NULL DEFAULT 0,
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (household_id, dedupe_key)
);
CREATE INDEX notification_status_idx ON notification (status, not_before);

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['task_list','task_definition','task_definition_candidate','task','task_assignee',
                             'task_step','member_away','routine','routine_member','routine_step','routine_run',
                             'routine_run_step','notification']
    LOOP
        IF t IN ('task_list','task_definition','task','member_away','routine','routine_run','notification') THEN
            EXECUTE format('CREATE TRIGGER %I BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION set_updated_at()', t || '_updated_at', t);
        END IF;
        EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I TO fa_app', t);
        EXECUTE format('CREATE POLICY tenant ON %I USING (household_id = app_household() OR app_is_system()) WITH CHECK (household_id = app_household() OR app_is_system())', t);
    END LOOP;
END
$$;
