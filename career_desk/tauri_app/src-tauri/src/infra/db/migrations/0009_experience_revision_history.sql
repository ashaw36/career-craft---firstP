CREATE TABLE experience_revisions (
  experience_id TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK(revision > 0),
  source TEXT NOT NULL CHECK(source IN ('migration','create','update','ai_enrichment','restore','delete')),
  snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
  deleted INTEGER NOT NULL DEFAULT 0 CHECK(deleted IN (0,1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(experience_id, revision)
);
CREATE INDEX idx_experience_revisions_recent ON experience_revisions(experience_id, revision DESC);
INSERT INTO experience_revisions(experience_id,revision,source,snapshot_json,deleted)
SELECT id,version,'migration',json_object(
 'id',id,'userId',user_id,'type',type,'title',title,'organization',organization,
 'startDate',start_date,'endDate',end_date,'rawDescription',raw_description,
 'structuredAchievements',json(structured_achievements),'skillsDemonstrated',json(skills_demonstrated),
 'status',status,'version',version,'industryTags',json(industry_tags),'educationLevel',education_level
),0 FROM experiences;
