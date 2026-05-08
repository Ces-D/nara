CREATE TABLE IF NOT EXISTS category (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS item (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    front       TEXT NOT NULL,
    back        TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS item_state (
    item_id          INTEGER PRIMARY KEY REFERENCES item(id) ON DELETE CASCADE,
    stability        REAL,
    difficulty       REAL,
    due_at           INTEGER NOT NULL,
    last_reviewed_at INTEGER,
    reps             INTEGER NOT NULL DEFAULT 0,
    lapses           INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS category_tag (
    name TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS category_tag_link (
    category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    tag_name    TEXT NOT NULL REFERENCES category_tag(name) ON DELETE CASCADE ON UPDATE CASCADE,
    PRIMARY KEY (category_id, tag_name)
);

CREATE TRIGGER IF NOT EXISTS delete_orphan_category_tag
AFTER DELETE ON category_tag_link
BEGIN
    DELETE FROM category_tag
    WHERE name = OLD.tag_name
      AND NOT EXISTS (SELECT 1 FROM category_tag_link WHERE tag_name = OLD.tag_name);
end
;

