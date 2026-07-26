ALTER TABLE experiences ADD COLUMN industry_tags TEXT NOT NULL DEFAULT '[]';
ALTER TABLE experiences ADD COLUMN education_level TEXT CHECK(education_level IS NULL OR education_level IN ('high_school','associate','bachelor','master','doctorate','other'));
ALTER TABLE job_descs ADD COLUMN industry_tags TEXT NOT NULL DEFAULT '[]';
ALTER TABLE job_descs ADD COLUMN education_levels TEXT NOT NULL DEFAULT '[]';
