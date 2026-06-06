CREATE TABLE IF NOT EXISTS category (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL,    -- describes the information this category holds
  created_at  INTEGER NOT NULL
);
