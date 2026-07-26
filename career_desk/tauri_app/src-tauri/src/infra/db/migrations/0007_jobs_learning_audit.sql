CREATE TABLE job_status_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  match_id TEXT NOT NULL REFERENCES job_matches(id) ON DELETE CASCADE,
  from_status TEXT,
  to_status TEXT NOT NULL,
  changed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK(to_status IN ('new','interested','applied','interviewing','offered','rejected','ghosted','accepted','declined'))
);
CREATE INDEX ix_job_status_events_match ON job_status_events(match_id,id);
INSERT INTO job_status_events(match_id,from_status,to_status,changed_at)
SELECT id,NULL,tracking_status,COALESCE(created_at,CURRENT_TIMESTAMP) FROM job_matches;
ALTER TABLE learning_paths ADD COLUMN context_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE learning_paths ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE learning_items ADD COLUMN source TEXT NOT NULL DEFAULT '';
ALTER TABLE learning_items ADD COLUMN resource_kind TEXT NOT NULL DEFAULT 'resource';
ALTER TABLE learning_items ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
