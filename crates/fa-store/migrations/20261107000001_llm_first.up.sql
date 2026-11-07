-- D-107: LLM-first mail processing, rules fallback and catch-up queue (plan/M4/T-22, arch/AI.md).

ALTER TABLE ai_settings
    ADD COLUMN mail_mode text NOT NULL DEFAULT 'llm_first' CHECK (mail_mode IN ('llm_first', 'rules_first')),
    ADD COLUMN catchup_days int NOT NULL DEFAULT 30 CHECK (catchup_days BETWEEN 0 AND 365),
    ADD COLUMN extract_fallback_cutoff real NOT NULL DEFAULT 0.8
        CHECK (extract_fallback_cutoff BETWEEN 0.5 AND 1);

ALTER TABLE message
    ADD COLUMN llm_state text NOT NULL DEFAULT 'none' CHECK (llm_state IN ('none', 'queued', 'done', 'awaiting'));
CREATE INDEX message_llm_awaiting_idx ON message (household_id, received_at DESC) WHERE llm_state = 'awaiting';
