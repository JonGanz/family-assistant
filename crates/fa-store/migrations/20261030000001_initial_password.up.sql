-- True while the password was set by another adult and the member has not chosen their own.
ALTER TABLE user_account ADD COLUMN initial_password boolean NOT NULL DEFAULT false;
