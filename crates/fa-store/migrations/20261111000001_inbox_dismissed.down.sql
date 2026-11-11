UPDATE event SET status = 'expired' WHERE status = 'dismissed';
UPDATE contact_suggestion SET status = 'expired' WHERE status = 'dismissed';
ALTER TABLE event DROP CONSTRAINT event_status_check;
ALTER TABLE event ADD CONSTRAINT event_status_check
    CHECK (status IN ('extracted', 'proposed', 'approved', 'written', 'rejected', 'expired'));
ALTER TABLE contact_suggestion DROP CONSTRAINT contact_suggestion_status_check;
ALTER TABLE contact_suggestion ADD CONSTRAINT contact_suggestion_status_check
    CHECK (status IN ('proposed', 'accepted', 'rejected', 'expired'));
