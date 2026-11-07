DROP INDEX message_llm_awaiting_idx;
ALTER TABLE message DROP COLUMN llm_state;
ALTER TABLE ai_settings
    DROP COLUMN extract_fallback_cutoff,
    DROP COLUMN catchup_days,
    DROP COLUMN mail_mode;
