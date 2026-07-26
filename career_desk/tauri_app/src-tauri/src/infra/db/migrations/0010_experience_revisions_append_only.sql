CREATE TRIGGER experience_revisions_no_update
BEFORE UPDATE ON experience_revisions
BEGIN
  SELECT RAISE(ABORT, 'experience revisions are append-only');
END;
CREATE TRIGGER experience_revisions_no_delete
BEFORE DELETE ON experience_revisions
BEGIN
  SELECT RAISE(ABORT, 'experience revisions are append-only');
END;
