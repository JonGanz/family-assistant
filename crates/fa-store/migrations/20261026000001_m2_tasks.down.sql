DROP TABLE notification, routine_run_step, routine_run, routine_step, routine_member, routine,
           member_away, task_step, task_assignee, task, task_definition_candidate, task_definition,
           task_list;
DROP INDEX notify_pref_member_uq;
ALTER TABLE notify_pref ALTER COLUMN triggers SET DEFAULT '{}';
ALTER TABLE notify_pref
    DROP COLUMN due_soon_minutes,
    DROP COLUMN nudge_minutes,
    DROP COLUMN quiet_from,
    DROP COLUMN quiet_until,
    DROP COLUMN copy_to_member_id;
ALTER TABLE weather_rule DROP COLUMN task_mode;
ALTER TABLE household
    DROP COLUMN now_list_size,
    DROP COLUMN now_weights,
    DROP COLUMN quiet_from,
    DROP COLUMN quiet_until;
