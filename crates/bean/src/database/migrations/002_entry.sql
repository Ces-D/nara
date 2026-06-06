CREATE TABLE IF NOT EXISTS entry (
  id          INTEGER PRIMARY KEY,
  category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,    -- one sentence summary of the content
  content     TEXT NOT NULL,    -- the text information being stored
  entry_date  INTEGER NOT NULL,    -- the date the information belongs to (YYYY-MM-DD)
  created_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entry_category ON entry(category_id);
CREATE INDEX IF NOT EXISTS idx_entry_date ON entry(entry_date);

