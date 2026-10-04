-- Bind each fenced claim to the credential that acquired it.
ALTER TABLE telegram_outbox ADD COLUMN claim_generation INTEGER
    REFERENCES telegram_credential_generations(generation);