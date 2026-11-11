-- Second LLM pass before a mail-extracted event is proposed: is it already on the calendar?
-- (arch/AI.md, "Event duplicate check"). On by default; the threshold is the least confidence
-- at which a "duplicate" answer drops the proposal.
ALTER TABLE ai_settings
    ADD COLUMN event_dedupe_llm boolean NOT NULL DEFAULT true,
    ADD COLUMN event_dedupe_threshold real NOT NULL DEFAULT 0.8
        CHECK (event_dedupe_threshold >= 0 AND event_dedupe_threshold <= 1);
