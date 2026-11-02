-- 0 = sealed without AAD (opened with an empty AAD); 1 = bound to its owner (C-10).
ALTER TABLE secret ADD COLUMN aad_version smallint NOT NULL DEFAULT 0;
ALTER TABLE instance_setting ADD COLUMN aad_version smallint NOT NULL DEFAULT 0;
