-- Secret values are stored exclusively by the platform SecretStore.
CREATE TABLE telegram_credential_generations (
    generation INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_identity TEXT NOT NULL,
    secret_reference TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK(status IN ('CANDIDATE', 'ACTIVE', 'RETIRED')),
    created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX telegram_single_active_credential
    ON telegram_credential_generations(status) WHERE status = 'ACTIVE';