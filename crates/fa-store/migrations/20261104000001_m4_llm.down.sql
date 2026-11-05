ALTER TABLE task DROP COLUMN extracted_by;
ALTER TABLE bill DROP COLUMN extracted_by;
ALTER TABLE event DROP COLUMN extracted_by;
DROP TABLE llm_call;
DROP TABLE ai_settings;
DROP TABLE llm_endpoint;
DELETE FROM secret WHERE kind = 'api_key';
ALTER TABLE secret DROP CONSTRAINT secret_kind_check;
ALTER TABLE secret ADD CONSTRAINT secret_kind_check CHECK (kind IN ('oauth_token', 'other'));
