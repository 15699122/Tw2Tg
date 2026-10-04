-- Grants bind exact outbox rows to a credential generation, not future rows.
CREATE TABLE telegram_resume_authorizations (
    generation INTEGER NOT NULL REFERENCES telegram_credential_generations(generation),
    outbox_id INTEGER NOT NULL REFERENCES telegram_outbox(id) ON DELETE CASCADE,
    granted_at TEXT NOT NULL,
    PRIMARY KEY(generation, outbox_id)
);