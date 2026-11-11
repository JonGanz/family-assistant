ALTER TABLE ai_settings DROP CONSTRAINT IF EXISTS ai_settings_mail_mode_check;
ALTER TABLE ai_settings
    ADD CONSTRAINT ai_settings_mail_mode_check CHECK (mail_mode IN ('llm_first', 'rules_first', 'llm_only'));
