CREATE TABLE IF NOT EXISTS resume_versions (
  id TEXT PRIMARY KEY NOT NULL,
  persona_id TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  template TEXT NOT NULL,
  revision INTEGER NOT NULL,
  data_json TEXT NOT NULL,
  parent_id TEXT REFERENCES resume_versions(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(persona_id, revision)
);
CREATE INDEX IF NOT EXISTS ix_resume_versions_persona_revision ON resume_versions(persona_id, revision DESC);
