CREATE TABLE IF NOT EXISTS schedule (
  id INTEGER PRIMARY KEY,
  name            TEXT NOT NULL,
  task_type       TEXT NOT NULL,    -- e.g. "dispatches.build", "workouts.build"
  payload         TEXT NOT NULL,    -- JSON for the handler
  rrule           TEXT,             -- NULL for one-shot
  at_unix         INTEGER,          -- non-null for one-shot
  next_run_unix   INTEGER,
  start_unix      INTEGER NOT NULL
);

