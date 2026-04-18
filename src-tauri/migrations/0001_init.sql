CREATE TABLE IF NOT EXISTS generation_batches (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    source_text TEXT NOT NULL,
    selected_keyword TEXT,
    context_title TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cards (
    id TEXT PRIMARY KEY,
    batch_id TEXT REFERENCES generation_batches(id) ON DELETE SET NULL,
    keyword TEXT NOT NULL,
    definition TEXT NOT NULL,
    explanation TEXT NOT NULL,
    source_excerpt TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    next_review_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_cards_batch_id ON cards(batch_id);
CREATE INDEX IF NOT EXISTS idx_cards_status ON cards(status);
CREATE INDEX IF NOT EXISTS idx_cards_next_review_at ON cards(next_review_at);

CREATE TABLE IF NOT EXISTS review_schedule (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    review_step INTEGER NOT NULL,
    due_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_review_schedule_due_at ON review_schedule(due_at);
CREATE INDEX IF NOT EXISTS idx_review_schedule_status ON review_schedule(status);
CREATE INDEX IF NOT EXISTS idx_review_schedule_card_id ON review_schedule(card_id);

CREATE TABLE IF NOT EXISTS review_logs (
    id TEXT PRIMARY KEY,
    review_schedule_id TEXT NOT NULL REFERENCES review_schedule(id) ON DELETE CASCADE,
    card_id TEXT NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    result TEXT NOT NULL,
    previous_step INTEGER NOT NULL,
    next_step INTEGER NOT NULL,
    reviewed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_review_logs_card_id ON review_logs(card_id);

CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    theme TEXT NOT NULL,
    language TEXT NOT NULL,
    notification_enabled INTEGER NOT NULL DEFAULT 1,
    review_reminder_enabled INTEGER NOT NULL DEFAULT 1,
    review_reminder_time TEXT NOT NULL,
    default_model_profile_id TEXT,
    export_directory TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS model_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    model TEXT,
    timeout_ms INTEGER NOT NULL,
    api_key_secret_ref TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
