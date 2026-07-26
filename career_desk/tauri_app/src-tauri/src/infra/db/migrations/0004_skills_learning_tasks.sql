CREATE TABLE IF NOT EXISTS custom_skills (
 id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, name TEXT NOT NULL, category TEXT NOT NULL,
 description TEXT NOT NULL DEFAULT '', aliases TEXT NOT NULL DEFAULT '[]', prerequisites TEXT NOT NULL DEFAULT '[]',
 level INTEGER NOT NULL CHECK(level BETWEEN 1 AND 3), resources TEXT NOT NULL DEFAULT '[]',
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE(owner_id,name COLLATE NOCASE)
);
CREATE TABLE IF NOT EXISTS learning_items (
 id TEXT PRIMARY KEY, path_id TEXT NOT NULL REFERENCES learning_paths(id) ON DELETE CASCADE,
 skill_id TEXT NOT NULL, title TEXT NOT NULL, resource_url TEXT, estimated_hours INTEGER NOT NULL DEFAULT 0,
 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','in_progress','completed','skipped')),
 completion_note TEXT, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS learning_conversions (
 item_id TEXT PRIMARY KEY REFERENCES learning_items(id) ON DELETE CASCADE,
 experience_id TEXT NOT NULL UNIQUE REFERENCES experiences(id) ON DELETE RESTRICT,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS task_metadata (
 id TEXT PRIMARY KEY, operation TEXT NOT NULL, state TEXT NOT NULL,
 progress REAL NOT NULL DEFAULT 0, error_code TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS ix_custom_skills_owner ON custom_skills(owner_id);
CREATE INDEX IF NOT EXISTS ix_learning_items_path ON learning_items(path_id);
