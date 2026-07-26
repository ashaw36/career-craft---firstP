//! SQLite adapters for CC-FR-001/002/004/005. JSON is compatible with legacy SQLAlchemy rows.
use crate::{
    application::ports::{ApplicationError, ExperienceRepository, PersonaRepository},
    domain::entities::*,
};
use rusqlite::{params, Connection, OptionalExtension, Row};

fn db(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Unavailable(error.to_string())
}
fn json<T: serde::Serialize>(value: &T) -> Result<String, ApplicationError> {
    serde_json::to_string(value).map_err(|e| ApplicationError::Validation(e.to_string()))
}
fn strings(value: Option<String>) -> Vec<String> {
    value
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn kind(value: &str) -> Result<ExperienceType, rusqlite::Error> {
    match value {
        "work" => Ok(ExperienceType::Work),
        "project" => Ok(ExperienceType::Project),
        "education" => Ok(ExperienceType::Education),
        "certification" => Ok(ExperienceType::Certification),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
fn kind_str(value: &ExperienceType) -> &'static str {
    match value {
        ExperienceType::Work => "work",
        ExperienceType::Project => "project",
        ExperienceType::Education => "education",
        ExperienceType::Certification => "certification",
    }
}
fn status(value: &str) -> Result<ExperienceStatus, rusqlite::Error> {
    match value {
        "draft" => Ok(ExperienceStatus::Draft),
        "confirmed" => Ok(ExperienceStatus::Confirmed),
        "discarded" => Ok(ExperienceStatus::Discarded),
        "archived" => Ok(ExperienceStatus::Archived),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
fn status_str(value: &ExperienceStatus) -> &'static str {
    match value {
        ExperienceStatus::Draft => "draft",
        ExperienceStatus::Confirmed => "confirmed",
        ExperienceStatus::Discarded => "discarded",
        ExperienceStatus::Archived => "archived",
    }
}

fn experience(row: &Row<'_>) -> rusqlite::Result<Experience> {
    Ok(Experience {
        id: row.get(0)?,
        user_id: row.get(1)?,
        kind: kind(&row.get::<_, String>(2)?)?,
        title: row.get(3)?,
        organization: row.get(4)?,
        start_date: row.get(5)?,
        end_date: row.get(6)?,
        raw_description: row.get(7)?,
        structured_achievements: strings(row.get(8)?),
        skills_demonstrated: strings(row.get(9)?),
        status: status(&row.get::<_, String>(10)?)?,
        version: row.get(11)?,
        industry_tags: strings(row.get(12)?),
        education_level: row
            .get::<_, Option<String>>(13)?
            .and_then(|v| EducationLevel::parse(&v)),
    })
}
const EXP_COLS: &str = "id,user_id,type,title,organization,start_date,end_date,raw_description,structured_achievements,skills_demonstrated,status,version,industry_tags,education_level";

pub struct SqliteExperienceRepository<'a> {
    connection: &'a Connection,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperienceRevision {
    pub experience_id: String,
    pub revision: u32,
    pub source: String,
    pub snapshot: serde_json::Value,
    pub deleted: bool,
    pub created_at: String,
}

pub(crate) fn append_experience_revision(
    connection: &Connection,
    id: &str,
    source: &str,
) -> Result<(), ApplicationError> {
    connection.execute(
        "INSERT INTO experience_revisions(experience_id,revision,source,snapshot_json,deleted)
         SELECT id,version,?2,json_object(
          'id',id,'userId',user_id,'type',type,'title',title,'organization',organization,
          'startDate',start_date,'endDate',end_date,'rawDescription',raw_description,
          'structuredAchievements',json(structured_achievements),'skillsDemonstrated',json(skills_demonstrated),
          'status',status,'version',version,'industryTags',json(industry_tags),'educationLevel',education_level
         ),0 FROM experiences WHERE id=?1",
        params![id, source],
    ).map_err(db)?;
    Ok(())
}
impl<'a> SqliteExperienceRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
    fn changed(&self, count: usize, id: &str) -> Result<Experience, ApplicationError> {
        if count == 0 {
            return if self.get(id)?.is_some() {
                Err(ApplicationError::Conflict(
                    "experience version changed".into(),
                ))
            } else {
                Err(ApplicationError::NotFound("experience".into()))
            };
        }
        self.get(id)?
            .ok_or_else(|| ApplicationError::NotFound("experience".into()))
    }

    pub fn recent_revisions(&self, id: &str) -> Result<Vec<ExperienceRevision>, ApplicationError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT experience_id,revision,source,snapshot_json,deleted,created_at
             FROM experience_revisions WHERE experience_id=?1 ORDER BY revision DESC LIMIT 3",
            )
            .map_err(db)?;
        let values = statement
            .query_map([id], |row| {
                let snapshot: String = row.get(3)?;
                Ok(ExperienceRevision {
                    experience_id: row.get(0)?,
                    revision: row.get(1)?,
                    source: row.get(2)?,
                    snapshot: serde_json::from_str(&snapshot).unwrap_or(serde_json::Value::Null),
                    deleted: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)?;
        Ok(values)
    }

    pub fn restore_revision(
        &self,
        id: &str,
        revision: u32,
        expected_version: u32,
    ) -> Result<Experience, ApplicationError> {
        let transaction = self.connection.unchecked_transaction().map_err(db)?;
        let snapshot: String = transaction.query_row(
            "SELECT snapshot_json FROM experience_revisions WHERE experience_id=?1 AND revision=?2 AND deleted=0",
            params![id, revision], |row| row.get(0),
        ).map_err(|error| if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
            ApplicationError::NotFound("experience revision".into())
        } else { db(error) })?;
        let count = transaction.execute(
            "UPDATE experiences SET type=json_extract(?1,'$.type'),title=json_extract(?1,'$.title'),organization=json_extract(?1,'$.organization'),start_date=json_extract(?1,'$.startDate'),end_date=json_extract(?1,'$.endDate'),raw_description=json_extract(?1,'$.rawDescription'),structured_achievements=json_extract(?1,'$.structuredAchievements'),skills_demonstrated=json_extract(?1,'$.skillsDemonstrated'),status=json_extract(?1,'$.status'),industry_tags=json_extract(?1,'$.industryTags'),education_level=json_extract(?1,'$.educationLevel'),version=version+1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND version=?3",
            params![snapshot, id, expected_version],
        ).map_err(db)?;
        if count == 1 {
            append_experience_revision(&transaction, id, "restore")?;
        }
        transaction.commit().map_err(db)?;
        self.changed(count, id)
    }
}
impl ExperienceRepository for SqliteExperienceRepository<'_> {
    fn list(&self, user_id: &str) -> Result<Vec<Experience>, ApplicationError> {
        let mut s=self.connection.prepare(&format!("SELECT {EXP_COLS} FROM experiences WHERE user_id=?1 ORDER BY COALESCE(start_date,'') DESC,created_at DESC")).map_err(db)?;
        let rows = s
            .query_map([user_id], experience)
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db);
        rows
    }
    fn list_confirmed(&self, user_id: &str) -> Result<Vec<Experience>, ApplicationError> {
        let mut s=self.connection.prepare(&format!("SELECT {EXP_COLS} FROM experiences WHERE user_id=?1 AND status='confirmed' ORDER BY COALESCE(start_date,'') DESC")).map_err(db)?;
        let rows = s
            .query_map([user_id], experience)
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db);
        rows
    }
    fn get(&self, id: &str) -> Result<Option<Experience>, ApplicationError> {
        self.connection
            .query_row(
                &format!("SELECT {EXP_COLS} FROM experiences WHERE id=?1"),
                [id],
                experience,
            )
            .optional()
            .map_err(db)
    }
    fn create(&self, e: &Experience) -> Result<(), ApplicationError> {
        let transaction = self.connection.unchecked_transaction().map_err(db)?;
        transaction.execute("INSERT INTO experiences(id,user_id,type,title,organization,start_date,end_date,raw_description,structured_achievements,skills_demonstrated,metrics,status,version,industry_tags,education_level) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'[]',?11,?12,?13,?14)",params![e.id,e.user_id,kind_str(&e.kind),e.title,e.organization,e.start_date,e.end_date,e.raw_description,json(&e.structured_achievements)?,json(&e.skills_demonstrated)?,status_str(&e.status),e.version,json(&e.industry_tags)?,e.education_level.as_ref().map(EducationLevel::as_str)]).map_err(|x| if matches!(x,rusqlite::Error::SqliteFailure(_,Some(ref m)) if m.contains("UNIQUE")){ApplicationError::Conflict("experience exists".into())}else{db(x)})?;
        append_experience_revision(&transaction, &e.id, "create")?;
        transaction.commit().map_err(db)?;
        Ok(())
    }
    fn update(
        &self,
        id: &str,
        v: u32,
        p: &ExperiencePatch,
    ) -> Result<Experience, ApplicationError> {
        let old = self
            .get(id)?
            .ok_or_else(|| ApplicationError::NotFound("experience".into()))?;
        let transaction = self.connection.unchecked_transaction().map_err(db)?;
        let count=transaction.execute("UPDATE experiences SET type=?1,title=?2,organization=?3,start_date=?4,end_date=?5,raw_description=?6,structured_achievements=?7,skills_demonstrated=?8,status=?9,industry_tags=?10,education_level=?11,version=version+1,updated_at=CURRENT_TIMESTAMP WHERE id=?12 AND version=?13",params![kind_str(p.kind.as_ref().unwrap_or(&old.kind)),p.title.as_ref().unwrap_or(&old.title),p.organization.clone().unwrap_or(old.organization),p.start_date.clone().unwrap_or(old.start_date),p.end_date.clone().unwrap_or(old.end_date),p.raw_description.as_ref().unwrap_or(&old.raw_description),json(p.structured_achievements.as_ref().unwrap_or(&old.structured_achievements))?,json(p.skills_demonstrated.as_ref().unwrap_or(&old.skills_demonstrated))?,status_str(p.status.as_ref().unwrap_or(&old.status)),json(&p.industry_tags.clone().unwrap_or(old.industry_tags))?,p.education_level.clone().unwrap_or(old.education_level).as_ref().map(EducationLevel::as_str),id,v]).map_err(db)?;
        if count == 1 {
            append_experience_revision(&transaction, id, "update")?;
        }
        transaction.commit().map_err(db)?;
        self.changed(count, id)
    }
    fn update_enrichment(
        &self,
        id: &str,
        v: u32,
        e: &ExperienceEnrichment,
    ) -> Result<Experience, ApplicationError> {
        let transaction = self.connection.unchecked_transaction().map_err(db)?;
        let count=transaction.execute("UPDATE experiences SET structured_achievements=?1,skills_demonstrated=?2,version=version+1,updated_at=CURRENT_TIMESTAMP WHERE id=?3 AND version=?4",params![json(&e.structured_achievements)?,json(&e.skills_demonstrated)?,id,v]).map_err(db)?;
        if count == 1 {
            append_experience_revision(&transaction, id, "ai_enrichment")?;
        }
        transaction.commit().map_err(db)?;
        self.changed(count, id)
    }
    fn delete(&self, id: &str, v: u32) -> Result<(), ApplicationError> {
        let transaction = self.connection.unchecked_transaction().map_err(db)?;
        let tombstone = transaction.execute(
            "INSERT INTO experience_revisions(experience_id,revision,source,snapshot_json,deleted)
             SELECT e.id,e.version+1,'delete',json_object(
              'id',e.id,'userId',e.user_id,'type',e.type,'title',e.title,'organization',e.organization,
              'startDate',e.start_date,'endDate',e.end_date,'rawDescription',e.raw_description,
              'structuredAchievements',json(e.structured_achievements),'skillsDemonstrated',json(e.skills_demonstrated),
              'status',e.status,'version',e.version+1,'industryTags',json(e.industry_tags),'educationLevel',e.education_level
             ),1 FROM experiences e WHERE e.id=?1 AND e.version=?2",
            params![id, v],
        ).map_err(db)?;
        let count = transaction
            .execute(
                "DELETE FROM experiences WHERE id=?1 AND version=?2",
                params![id, v],
            )
            .map_err(db)?;
        debug_assert_eq!(tombstone, count);
        transaction.commit().map_err(db)?;
        if count == 0 {
            return if self.get(id)?.is_some() {
                Err(ApplicationError::Conflict(
                    "experience version changed".into(),
                ))
            } else {
                Err(ApplicationError::NotFound("experience".into()))
            };
        }
        Ok(())
    }
}

pub struct SqlitePersonaRepository<'a> {
    connection: &'a Connection,
}
impl<'a> SqlitePersonaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}
fn persona(row: &Row<'_>) -> rusqlite::Result<Persona> {
    let weights: String = row.get(7)?;
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&weights).unwrap_or_default();
    Ok(Persona {
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        is_default: row.get(3)?,
        identity_statement: row.get(4)?,
        career_narrative: row.get(5)?,
        tone_style: row.get(6)?,
        capability_weights: map
            .into_iter()
            .filter_map(|(k, v)| v.as_f64().map(|n| (k, n)))
            .collect(),
        target_job_profiles: strings(row.get(8)?),
        max_experiences: row.get(9)?,
        preferred_model: row.get(10)?,
    })
}
const PER_COLS:&str="id,user_id,name,is_default,identity_statement,career_narrative,tone_style,capability_weights,target_job_profiles,max_experiences,preferred_model";
fn weight(row: &Row<'_>) -> rusqlite::Result<RoleExperienceWeight> {
    Ok(RoleExperienceWeight {
        id: row.get(0)?,
        persona_id: row.get(1)?,
        experience_id: row.get(2)?,
        relevance_score: row.get(3)?,
        reframed_summary: row.get(4)?,
        highlighted_skills: strings(row.get(5)?),
        user_overridden: row.get(6)?,
    })
}
fn weight_id(p: &str, e: &str) -> String {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    let mut h = DefaultHasher::new();
    p.hash(&mut h);
    e.hash(&mut h);
    format!("w-{:016x}", h.finish())
}
impl PersonaRepository for SqlitePersonaRepository<'_> {
    fn list(&self, user: &str) -> Result<Vec<Persona>, ApplicationError> {
        let mut s=self.connection.prepare(&format!("SELECT {PER_COLS} FROM personas WHERE user_id=?1 ORDER BY is_default DESC,created_at")).map_err(db)?;
        let rows = s
            .query_map([user], persona)
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db);
        rows
    }
    fn get(&self, id: &str) -> Result<Option<Persona>, ApplicationError> {
        self.connection
            .query_row(
                &format!("SELECT {PER_COLS} FROM personas WHERE id=?1"),
                [id],
                persona,
            )
            .optional()
            .map_err(db)
    }
    fn create(&self, p: &Persona) -> Result<(), ApplicationError> {
        let weights: serde_json::Map<String, serde_json::Value> = p
            .capability_weights
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!(v)))
            .collect();
        self.connection.execute("INSERT INTO personas(id,user_id,name,is_default,identity_statement,career_narrative,tone_style,capability_weights,target_job_profiles,max_experiences,preferred_model) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![p.id,p.user_id,p.name,p.is_default,p.identity_statement,p.career_narrative,p.tone_style,json(&weights)?,json(&p.target_job_profiles)?,p.max_experiences,p.preferred_model]).map_err(db)?;
        Ok(())
    }
    fn update(&self, id: &str, p: &PersonaPatch) -> Result<Persona, ApplicationError> {
        let old = self
            .get(id)?
            .ok_or_else(|| ApplicationError::NotFound("persona".into()))?;
        let weights = p
            .capability_weights
            .clone()
            .unwrap_or(old.capability_weights);
        let map: serde_json::Map<String, serde_json::Value> = weights
            .into_iter()
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect();
        let n=self.connection.execute("UPDATE personas SET name=?1,identity_statement=?2,career_narrative=?3,tone_style=?4,capability_weights=?5,target_job_profiles=?6,max_experiences=?7,preferred_model=?8,updated_at=CURRENT_TIMESTAMP WHERE id=?9",params![p.name.as_ref().unwrap_or(&old.name),p.identity_statement.clone().unwrap_or(old.identity_statement),p.career_narrative.clone().unwrap_or(old.career_narrative),p.tone_style.clone().unwrap_or(old.tone_style),json(&map)?,json(&p.target_job_profiles.clone().unwrap_or(old.target_job_profiles))?,p.max_experiences.unwrap_or(old.max_experiences),p.preferred_model.clone().unwrap_or(old.preferred_model),id]).map_err(db)?;
        if n == 0 {
            return Err(ApplicationError::NotFound("persona".into()));
        }
        self.get(id)?
            .ok_or_else(|| ApplicationError::NotFound("persona".into()))
    }
    fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        if self
            .connection
            .execute("DELETE FROM personas WHERE id=?1", [id])
            .map_err(db)?
            == 0
        {
            return Err(ApplicationError::NotFound("persona".into()));
        }
        Ok(())
    }
    fn get_weights(&self, p: &str) -> Result<Vec<RoleExperienceWeight>, ApplicationError> {
        let mut s=self.connection.prepare("SELECT id,persona_id,experience_id,relevance_score,reframed_summary,highlighted_skills,user_overridden FROM role_experience_weights WHERE persona_id=?1 ORDER BY relevance_score DESC").map_err(db)?;
        let rows = s
            .query_map([p], weight)
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db);
        rows
    }
    fn save_weights(&self, ws: &[RoleExperienceWeight]) -> Result<(), ApplicationError> {
        for w in ws {
            let id = if w.id.is_empty() {
                weight_id(&w.persona_id, &w.experience_id)
            } else {
                w.id.clone()
            };
            self.connection.execute("INSERT INTO role_experience_weights(id,persona_id,experience_id,relevance_score,reframed_summary,highlighted_skills,user_overridden) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET relevance_score=CASE WHEN user_overridden THEN relevance_score ELSE excluded.relevance_score END,user_overridden=CASE WHEN user_overridden THEN 1 ELSE excluded.user_overridden END,updated_at=CURRENT_TIMESTAMP",params![id,w.persona_id,w.experience_id,w.relevance_score,w.reframed_summary,json(&w.highlighted_skills)?,w.user_overridden]).map_err(db)?;
        }
        Ok(())
    }
    fn override_weight(
        &self,
        p: &str,
        e: &str,
        s: f64,
    ) -> Result<RoleExperienceWeight, ApplicationError> {
        let existing:Option<String>=self.connection.query_row("SELECT id FROM role_experience_weights WHERE persona_id=?1 AND experience_id=?2 ORDER BY created_at LIMIT 1",params![p,e],|r|r.get(0)).optional().map_err(db)?;
        let id = existing.unwrap_or_else(|| weight_id(p, e));
        self.connection.execute("INSERT INTO role_experience_weights(id,persona_id,experience_id,relevance_score,highlighted_skills,user_overridden) VALUES(?1,?2,?3,?4,'[]',1) ON CONFLICT(id) DO UPDATE SET relevance_score=excluded.relevance_score,user_overridden=1,updated_at=CURRENT_TIMESTAMP",params![id,p,e,s]).map_err(db)?;
        self.connection.query_row("SELECT id,persona_id,experience_id,relevance_score,reframed_summary,highlighted_skills,user_overridden FROM role_experience_weights WHERE id=?1",[id],weight).map_err(db)
    }
    fn reset_weight(&self, p: &str, e: &str) -> Result<(), ApplicationError> {
        if self.connection.execute("UPDATE role_experience_weights SET user_overridden=0 WHERE persona_id=?1 AND experience_id=?2",params![p,e]).map_err(db)?==0{return Err(ApplicationError::NotFound("fit score".into()));}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connection() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        c.execute_batch(include_str!(
            "../../../../contracts/db/legacy_schema_v1.sql"
        ))
        .unwrap();
        c.execute_batch(include_str!(
            "../db/migrations/0005_structured_evidence.sql"
        ))
        .unwrap();
        c.execute_batch(include_str!(
            "../db/migrations/0009_experience_revision_history.sql"
        ))
        .unwrap();
        c
    }
    fn experience() -> Experience {
        Experience {
            id: "e1".into(),
            user_id: "default".into(),
            kind: ExperienceType::Work,
            title: "工程师".into(),
            organization: None,
            start_date: Some("2024-01-01".into()),
            end_date: None,
            raw_description: "用户原始事实".into(),
            structured_achievements: vec![],
            skills_demonstrated: vec!["Rust".into()],
            industry_tags: vec![],
            education_level: None,
            status: ExperienceStatus::Confirmed,
            version: 1,
        }
    }
    fn persona_value() -> Persona {
        Persona {
            id: "p1".into(),
            user_id: "default".into(),
            name: "技术".into(),
            is_default: true,
            identity_statement: None,
            career_narrative: None,
            tone_style: None,
            capability_weights: vec![("Rust".into(), 1.0)],
            target_job_profiles: vec![],
            max_experiences: 5,
            preferred_model: None,
        }
    }

    #[test]
    fn ai_enrichment_preserves_raw_and_increments_version() {
        let c = connection();
        let r = SqliteExperienceRepository::new(&c);
        r.create(&experience()).unwrap();
        let updated = r
            .update_enrichment(
                "e1",
                1,
                &ExperienceEnrichment {
                    structured_achievements: vec!["提升 30%".into()],
                    skills_demonstrated: vec!["Rust".into()],
                },
            )
            .unwrap();
        assert_eq!(updated.raw_description, "用户原始事实");
        assert_eq!(updated.version, 2);
    }
    #[test]
    fn stale_experience_update_is_conflict() {
        let c = connection();
        let r = SqliteExperienceRepository::new(&c);
        r.create(&experience()).unwrap();
        r.update(
            "e1",
            1,
            &ExperiencePatch {
                title: Some("高级工程师".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(
            r.update("e1", 1, &ExperiencePatch::default()),
            Err(ApplicationError::Conflict(_))
        ));
    }
    #[test]
    fn deleting_persona_does_not_delete_experience() {
        let c = connection();
        let er = SqliteExperienceRepository::new(&c);
        let pr = SqlitePersonaRepository::new(&c);
        er.create(&experience()).unwrap();
        pr.create(&persona_value()).unwrap();
        pr.delete("p1").unwrap();
        assert!(er.get("e1").unwrap().is_some());
    }
    #[test]
    fn override_and_reset_are_persisted() {
        let c = connection();
        let er = SqliteExperienceRepository::new(&c);
        let pr = SqlitePersonaRepository::new(&c);
        er.create(&experience()).unwrap();
        pr.create(&persona_value()).unwrap();
        let w = pr.override_weight("p1", "e1", 0.25).unwrap();
        assert!(w.user_overridden);
        pr.reset_weight("p1", "e1").unwrap();
        assert!(!pr.get_weights("p1").unwrap()[0].user_overridden);
    }
    #[test]
    fn experience_crud_lists_statuses_and_reports_delete_errors() {
        let c = connection();
        let r = SqliteExperienceRepository::new(&c);
        let variants = [
            (ExperienceType::Work, "work"),
            (ExperienceType::Project, "project"),
            (ExperienceType::Education, "education"),
            (ExperienceType::Certification, "certification"),
        ];
        for (index, (kind_value, _)) in variants.into_iter().enumerate() {
            let mut value = experience();
            value.id = format!("e{index}");
            value.kind = kind_value;
            value.status = if index == 0 {
                ExperienceStatus::Confirmed
            } else if index == 1 {
                ExperienceStatus::Draft
            } else if index == 2 {
                ExperienceStatus::Discarded
            } else {
                ExperienceStatus::Archived
            };
            r.create(&value).unwrap()
        }
        assert_eq!(r.list("default").unwrap().len(), 4);
        assert_eq!(r.list_confirmed("default").unwrap().len(), 1);
        assert!(matches!(
            r.create(&experience()),
            Err(ApplicationError::Conflict(_))
        ));
        assert!(matches!(
            r.update("missing", 1, &ExperiencePatch::default()),
            Err(ApplicationError::NotFound(_))
        ));
        assert!(matches!(
            r.update_enrichment("missing", 1, &ExperienceEnrichment::default()),
            Err(ApplicationError::NotFound(_))
        ));
        assert!(matches!(
            r.delete("e0", 99),
            Err(ApplicationError::Conflict(_))
        ));
        r.delete("e0", 1).unwrap();
        assert!(matches!(
            r.delete("e0", 1),
            Err(ApplicationError::NotFound(_))
        ));
    }
    #[test]
    fn revision_history_is_append_only_restorable_cas_safe_and_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(include_str!(
                "../../../../contracts/db/legacy_schema_v1.sql"
            ))
            .unwrap();
            c.execute_batch(include_str!(
                "../db/migrations/0005_structured_evidence.sql"
            ))
            .unwrap();
            c.execute_batch(include_str!(
                "../db/migrations/0009_experience_revision_history.sql"
            ))
            .unwrap();
            let repository = SqliteExperienceRepository::new(&c);
            repository.create(&experience()).unwrap();
            let updated = repository
                .update(
                    "e1",
                    1,
                    &ExperiencePatch {
                        title: Some("Lead".into()),
                        ..Default::default()
                    },
                )
                .unwrap();
            repository
                .update_enrichment(
                    "e1",
                    updated.version,
                    &ExperienceEnrichment {
                        structured_achievements: vec!["Shipped".into()],
                        skills_demonstrated: vec!["Rust".into()],
                    },
                )
                .unwrap();
            assert!(matches!(
                repository.restore_revision("e1", 1, 2),
                Err(ApplicationError::Conflict(_))
            ));
            let restored = repository.restore_revision("e1", 1, 3).unwrap();
            assert_eq!(
                (restored.title.as_str(), restored.version),
                (experience().title.as_str(), 4)
            );
            let recent = repository.recent_revisions("e1").unwrap();
            assert_eq!(recent.len(), 3);
            assert_eq!(recent[0].source, "restore");
        }
        let reopened = Connection::open(&path).unwrap();
        let repository = SqliteExperienceRepository::new(&reopened);
        assert_eq!(repository.recent_revisions("e1").unwrap()[0].revision, 4);
        repository.delete("e1", 4).unwrap();
        let deleted = repository.recent_revisions("e1").unwrap();
        assert_eq!(
            (deleted[0].source.as_str(), deleted[0].deleted),
            ("delete", true)
        );
        assert!(repository.get("e1").unwrap().is_none());
    }
    #[test]
    fn delete_safely_creates_tombstone_when_legacy_row_has_no_revision() {
        let c = connection();
        c.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('orphan','u','work','T','R','draft',3)", []).unwrap();
        let repository = SqliteExperienceRepository::new(&c);
        repository.delete("orphan", 3).unwrap();
        let revision = repository.recent_revisions("orphan").unwrap();
        assert_eq!((revision[0].revision, revision[0].deleted), (4, true));
    }
    #[test]
    fn malformed_json_defaults_and_invalid_enums_are_unavailable() {
        let c = connection();
        let r = SqliteExperienceRepository::new(&c);
        r.create(&experience()).unwrap();
        c.execute("UPDATE experiences SET structured_achievements='not-json',skills_demonstrated=NULL WHERE id='e1'",[]).unwrap();
        let value = r.get("e1").unwrap().unwrap();
        assert!(value.structured_achievements.is_empty());
        assert!(value.skills_demonstrated.is_empty());
        c.execute("UPDATE experiences SET type='invalid' WHERE id='e1'", [])
            .unwrap();
        assert!(matches!(r.get("e1"), Err(ApplicationError::Unavailable(_))));
        c.execute(
            "UPDATE experiences SET type='work',status='invalid' WHERE id='e1'",
            [],
        )
        .unwrap();
        assert!(matches!(r.get("e1"), Err(ApplicationError::Unavailable(_))));
    }
    #[test]
    fn persona_crud_defaults_bad_json_and_reports_missing() {
        let c = connection();
        let r = SqlitePersonaRepository::new(&c);
        r.create(&persona_value()).unwrap();
        assert_eq!(r.list("default").unwrap().len(), 1);
        let updated = r
            .update(
                "p1",
                &PersonaPatch {
                    name: Some("Lead".into()),
                    identity_statement: Some(Some("Identity".into())),
                    career_narrative: Some(Some("Story".into())),
                    tone_style: Some(Some("Direct".into())),
                    capability_weights: Some(vec![("SQL".into(), 0.8)]),
                    target_job_profiles: Some(vec!["Staff".into()]),
                    max_experiences: Some(3),
                    preferred_model: Some(Some("model".into())),
                },
            )
            .unwrap();
        assert_eq!(updated.name, "Lead");
        assert_eq!(updated.capability_weights, vec![("SQL".into(), 0.8)]);
        c.execute("UPDATE personas SET capability_weights='bad-json',target_job_profiles='bad-json' WHERE id='p1'",[]).unwrap();
        let bad = r.get("p1").unwrap().unwrap();
        assert!(bad.capability_weights.is_empty());
        assert!(bad.target_job_profiles.is_empty());
        assert!(matches!(
            r.update("missing", &PersonaPatch::default()),
            Err(ApplicationError::NotFound(_))
        ));
        assert!(matches!(
            r.delete("missing"),
            Err(ApplicationError::NotFound(_))
        ));
        r.delete("p1").unwrap();
    }
    #[test]
    fn computed_weight_id_and_user_override_are_stable() {
        let c = connection();
        let er = SqliteExperienceRepository::new(&c);
        let pr = SqlitePersonaRepository::new(&c);
        er.create(&experience()).unwrap();
        pr.create(&persona_value()).unwrap();
        let auto = RoleExperienceWeight {
            id: String::new(),
            persona_id: "p1".into(),
            experience_id: "e1".into(),
            relevance_score: 0.4,
            reframed_summary: Some("fit".into()),
            highlighted_skills: vec!["Rust".into()],
            user_overridden: false,
        };
        pr.save_weights(std::slice::from_ref(&auto)).unwrap();
        let first = pr.get_weights("p1").unwrap()[0].clone();
        assert!(first.id.starts_with("w-"));
        pr.override_weight("p1", "e1", 0.9).unwrap();
        let mut generated = auto;
        generated.relevance_score = 0.1;
        pr.save_weights(&[generated]).unwrap();
        let preserved = pr.get_weights("p1").unwrap()[0].clone();
        assert_eq!(preserved.relevance_score, 0.9);
        assert!(preserved.user_overridden);
        assert!(matches!(
            pr.reset_weight("p1", "missing"),
            Err(ApplicationError::NotFound(_))
        ));
    }
    #[test]
    fn structured_evidence_round_trips_and_bad_json_defaults() {
        let c = connection();
        let r = SqliteExperienceRepository::new(&c);
        let mut value = experience();
        value.industry_tags = vec!["fintech".into()];
        value.education_level = Some(EducationLevel::Master);
        r.create(&value).unwrap();
        let stored = r.get("e1").unwrap().unwrap();
        assert_eq!(stored.industry_tags, vec!["fintech"]);
        assert_eq!(stored.education_level, Some(EducationLevel::Master));
        c.execute(
            "UPDATE experiences SET industry_tags='bad' WHERE id='e1'",
            [],
        )
        .unwrap();
        assert!(r.get("e1").unwrap().unwrap().industry_tags.is_empty());
        assert!(c
            .execute(
                "UPDATE experiences SET education_level='invalid' WHERE id='e1'",
                []
            )
            .is_err())
    }
}
