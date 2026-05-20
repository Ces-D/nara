CREATE TABLE IF NOT EXISTS job (
  id              INTEGER PRIMARY KEY,
  schedule_id     INTEGER,          -- nullable: ad-hoc jobs allowed
  parent_job_id   INTEGER,          -- the spawn link <-- KEY FIELD
  task_type       TEXT NOT NULL,
  payload         TEXT NOT NULL,
  status          INTEGER NOT NULL, -- 0=pending, 1=running, 2=completed, 3=failed
  artifact_ref    TEXT,             -- e.g. file path produced by parent
  due_unix        INTEGER NOT NULL,
  created_at      INTEGER NOT NULL,
  finished_at     INTEGER
);
