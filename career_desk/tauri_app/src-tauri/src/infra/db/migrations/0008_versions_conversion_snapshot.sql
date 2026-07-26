ALTER TABLE job_matches ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

ALTER TABLE learning_conversions RENAME TO learning_conversions_v7;
CREATE TABLE learning_conversions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id TEXT UNIQUE REFERENCES learning_items(id) ON DELETE SET NULL,
  experience_id TEXT UNIQUE REFERENCES experiences(id) ON DELETE SET NULL,
  source_item_id TEXT NOT NULL,
  source_path_id TEXT NOT NULL,
  source_skill_id TEXT NOT NULL,
  source_title TEXT NOT NULL,
  completion_note_snapshot TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO learning_conversions(item_id,experience_id,source_item_id,source_path_id,source_skill_id,source_title,completion_note_snapshot,created_at)
SELECT c.item_id,c.experience_id,c.item_id,i.path_id,i.skill_id,i.title,COALESCE(i.completion_note,''),c.created_at
FROM learning_conversions_v7 c JOIN learning_items i ON i.id=c.item_id;
DROP TABLE learning_conversions_v7;
CREATE INDEX ix_learning_conversions_path_snapshot ON learning_conversions(source_path_id);
