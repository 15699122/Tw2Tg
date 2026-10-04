CREATE TABLE telegram_rotation_decisions (
    generation INTEGER PRIMARY KEY REFERENCES telegram_credential_generations(generation),
    policy TEXT NOT NULL CHECK(policy IN ('automatic', 'confirm')),
    candidate_ids_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);