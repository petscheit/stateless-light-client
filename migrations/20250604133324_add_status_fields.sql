-- Add migration script here

ALTER TABLE epoch_updates ADD COLUMN status TEXT NOT NULL DEFAULT 'fetching';
ALTER TABLE epoch_updates ADD COLUMN error_reason TEXT;
