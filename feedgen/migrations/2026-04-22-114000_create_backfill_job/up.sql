CREATE TABLE backfill_job (
    id SERIAL PRIMARY KEY,
    did TEXT NOT NULL UNIQUE,
    state TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_backfill_job_state ON backfill_job(state);
