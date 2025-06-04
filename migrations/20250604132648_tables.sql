-- Add migration script here

CREATE TABLE proofs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proof TEXT NOT NULL -- JSON stored as text
);

CREATE TABLE epoch_updates (
    uuid TEXT PRIMARY KEY,
    epoch_number INTEGER NOT NULL,
    slot_number INTEGER NOT NULL UNIQUE,
    outputs TEXT, -- JSON stored as text
    atlantic_id TEXT,
    proof_id INTEGER,
    FOREIGN KEY (proof_id) REFERENCES proofs(id)
);

CREATE INDEX idx_epoch_updates_slot ON epoch_updates(slot_number);
CREATE INDEX idx_epoch_updates_epoch ON epoch_updates(epoch_number);
