CREATE TABLE turn_steps (
    turn_id TEXT NOT NULL,
    step_index INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('model_call', 'tool_call', 'approval')),
    has_side_effect INTEGER NOT NULL CHECK (has_side_effect IN (0, 1)),
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'cancelled_unknown')),
    attempt INTEGER NOT NULL DEFAULT 1,
    input TEXT NOT NULL,
    output TEXT,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT,
    PRIMARY KEY (turn_id, step_index)
);

UPDATE schema_version SET version = 4 WHERE id = 1;
