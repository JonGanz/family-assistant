-- Discard: "not needed" is not a verdict on the finding, so it records no human decision.
ALTER TABLE event DROP CONSTRAINT event_status_check;
ALTER TABLE event ADD CONSTRAINT event_status_check
    CHECK (status IN ('extracted', 'proposed', 'approved', 'written', 'rejected', 'expired', 'dismissed'));
ALTER TABLE contact_suggestion DROP CONSTRAINT contact_suggestion_status_check;
ALTER TABLE contact_suggestion ADD CONSTRAINT contact_suggestion_status_check
    CHECK (status IN ('proposed', 'accepted', 'rejected', 'expired', 'dismissed'));
