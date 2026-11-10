-- Whether the model may reason before answering (Settings -> AI; arch/AI.md). Off by default: a thinking
-- model spends its output budget and minutes on reasoning that classification and extraction rarely need.
ALTER TABLE llm_endpoint ADD COLUMN thinking boolean NOT NULL DEFAULT false;
