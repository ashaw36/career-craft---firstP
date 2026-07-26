# WP8 skills migration specification

Requirement mapping: CC-FR-013..017. This is an exact contract for a later numbered migration; this module does not edit the shared migration chain.

## Migration `0002_skills_learning_progress.sql`

Run inside one immediate transaction after a verified database backup.

```sql
ALTER TABLE skill_nodes ADD COLUMN origin TEXT NOT NULL DEFAULT 'built_in'
  CHECK (origin IN ('built_in', 'custom'));
ALTER TABLE skill_nodes ADD COLUMN owner_id TEXT;
ALTER TABLE skill_nodes ADD COLUMN prerequisites JSON NOT NULL DEFAULT '[]';
CREATE INDEX IF NOT EXISTS ix_skill_nodes_owner ON skill_nodes(owner_id, origin);

CREATE TABLE learning_item_conversions (
  learning_path_id VARCHAR(36) NOT NULL,
  item_id VARCHAR(100) NOT NULL,
  experience_id VARCHAR(36) NOT NULL UNIQUE,
  converted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (learning_path_id, item_id),
  FOREIGN KEY (learning_path_id) REFERENCES learning_paths(id) ON DELETE CASCADE,
  FOREIGN KEY (experience_id) REFERENCES experiences(id) ON DELETE CASCADE
);
```

After adding columns, upsert the 51 bundled nodes by stable `id`. Preserve user-edited legacy descriptions/resources when non-empty; populate missing aliases, prerequisites and resources from the versioned full catalog. A custom skill must use `origin='custom'` and a non-empty `owner_id`; application validation rejects updates/deletes of `built_in` rows.

Completion-to-experience is one transaction: verify the JSON learning item is completed and has a completion note, insert a draft experience whose immutable `raw_description` is that note, insert `learning_item_conversions`, then write `converted_experience_id` into the path JSON. The composite primary key provides exactly-once behavior.

Rollback restores the pre-migration backup because SQLite cannot safely drop the added columns on every supported runtime. Startup integrity checks must confirm 51 distinct built-ins, no dangling prerequisites, and no duplicate conversion rows.
