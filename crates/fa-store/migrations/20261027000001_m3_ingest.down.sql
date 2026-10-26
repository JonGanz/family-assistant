SELECT set_config('app.system', 'on', true);

-- Rows that use the new enum values cannot satisfy the old CHECKs.
DELETE FROM event WHERE status IN ('extracted', 'expired');
DROP TABLE event_message;
DROP INDEX event_extracted_status_idx, event_message_idx;
ALTER TABLE event
    DROP COLUMN message_id,
    DROP COLUMN org_id,
    DROP COLUMN target_calendar_id,
    DROP COLUMN route_confidence,
    DROP COLUMN pending_cancel,
    DROP COLUMN write_error,
    DROP COLUMN decided_by,
    DROP COLUMN decided_at;
ALTER TABLE event DROP CONSTRAINT event_status_check;
ALTER TABLE event ADD CONSTRAINT event_status_check CHECK (status IN ('proposed', 'approved', 'written', 'rejected'));

DROP TABLE mail_action, bill, category, mail_rule, decision, pipeline_stage, pipeline_version, decision_point,
           contact_suggestion, message_member, message, mail_folder_state, mail_source_owner, mail_source,
           enrollment_contact, enrollment, org_contact, org;

DELETE FROM notification WHERE trigger IN ('approval_needed', 'mail_time_sensitive');
ALTER TABLE notification DROP CONSTRAINT notification_trigger_check;
ALTER TABLE notification ADD CONSTRAINT notification_trigger_check
    CHECK (trigger IN ('assigned', 'overdue', 'digest', 'due_soon', 'routine_nudge', 'source_reauth'));
DELETE FROM task WHERE origin = 'email';
ALTER TABLE task DROP CONSTRAINT task_origin_check;
ALTER TABLE task ADD CONSTRAINT task_origin_check CHECK (origin IN ('manual', 'system', 'weather'));
DELETE FROM task_list WHERE kind = 'email';
ALTER TABLE task_list DROP CONSTRAINT task_list_kind_check;
ALTER TABLE task_list ADD CONSTRAINT task_list_kind_check CHECK (kind IN ('chores', 'homework', 'general'));

ALTER TABLE notify_pref DROP COLUMN approval_mode;
ALTER TABLE member DROP COLUMN aliases, DROP COLUMN gender, DROP COLUMN relevance_tags;
ALTER TABLE household
    DROP COLUMN currency,
    DROP COLUMN gender_terms,
    DROP COLUMN disabled_signals,
    DROP COLUMN events_auto_write,
    DROP COLUMN events_extract_threshold,
    DROP COLUMN events_route_threshold,
    DROP COLUMN bill_lead_days,
    DROP COLUMN inbox_task_expiry_days,
    DROP COLUMN inbox_contact_expiry_days,
    DROP COLUMN mail_content_policy,
    DROP COLUMN mail_examples_per_category,
    DROP COLUMN mail_max_pending_days;
