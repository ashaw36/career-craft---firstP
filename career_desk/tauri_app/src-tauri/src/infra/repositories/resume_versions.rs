use crate::{
    domain::resume::{ResumeEntry, ResumeHeader, ResumeRenderData, ResumeTemplate, ResumeVersion},
    error::AppError,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

pub const MAX_RESUME_VERSIONS: usize = 5;
pub struct SqliteResumeVersionRepository<'a> {
    connection: &'a Connection,
}
impl<'a> SqliteResumeVersionRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
    pub fn list(&self, persona_id: &str) -> Result<Vec<ResumeVersion>, AppError> {
        let mut s=self.connection.prepare("SELECT id,persona_id,label,template,revision,data_json,parent_id,created_at FROM resume_versions WHERE persona_id=?1 ORDER BY revision")?;
        let rows = s
            .query_map([persona_id], row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn latest(&self, persona_id: &str) -> Result<Option<ResumeVersion>, AppError> {
        self.connection.query_row("SELECT id,persona_id,label,template,revision,data_json,parent_id,created_at FROM resume_versions WHERE persona_id=?1 ORDER BY revision DESC LIMIT 1",[persona_id],row).optional().map_err(AppError::from)
    }
    pub fn get(&self, id: &str) -> Result<Option<ResumeVersion>, AppError> {
        self.connection.query_row("SELECT id,persona_id,label,template,revision,data_json,parent_id,created_at FROM resume_versions WHERE id=?1",[id],row).optional().map_err(AppError::from)
    }
    pub fn save(&self, version: &ResumeVersion) -> Result<ResumeVersion, AppError> {
        let transaction = self.connection.unchecked_transaction()?;
        let revision: u32 = transaction.query_row(
            "SELECT COALESCE(MAX(revision),0)+1 FROM resume_versions WHERE persona_id=?1",
            [&version.persona_id],
            |r| r.get(0),
        )?;
        let count: usize = transaction.query_row(
            "SELECT COUNT(*) FROM resume_versions WHERE persona_id=?1",
            [&version.persona_id],
            |r| r.get(0),
        )?;
        let mut stored = version.clone();
        stored.revision = revision;
        transaction.execute("INSERT INTO resume_versions(id,persona_id,label,template,revision,data_json,parent_id,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![stored.id,stored.persona_id,stored.label,stored.template.id(),stored.revision,data_json(&stored.data).to_string(),stored.parent_id,stored.created_at])?;
        if count >= MAX_RESUME_VERSIONS {
            transaction.execute("DELETE FROM resume_versions WHERE id=(SELECT id FROM resume_versions WHERE persona_id=?1 ORDER BY revision LIMIT 1)",[&stored.persona_id])?;
        }
        transaction.commit()?;
        Ok(stored)
    }
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ResumeVersion> {
    let template: String = r.get(3)?;
    let raw: String = r.get(5)?;
    let data = serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|v| parse_data(&v))
        .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
    Ok(ResumeVersion {
        id: r.get(0)?,
        persona_id: r.get(1)?,
        label: r.get(2)?,
        template: ResumeTemplate::ALL
            .into_iter()
            .find(|t| t.id() == template)
            .ok_or(rusqlite::Error::InvalidQuery)?,
        revision: r.get(4)?,
        data,
        parent_id: r.get(6)?,
        created_at: r.get(7)?,
    })
}
fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
fn optional(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_owned)
}
fn entry_json(e: &ResumeEntry) -> Value {
    json!({"sourceExperienceId":e.source_experience_id,"title":e.title,"organization":e.organization,"period":e.period,"summary":e.summary,"achievements":e.achievements,"skills":e.skills})
}
fn parse_entry(v: &Value) -> Option<ResumeEntry> {
    Some(ResumeEntry {
        source_experience_id: v.get("sourceExperienceId")?.as_str()?.into(),
        title: v.get("title")?.as_str()?.into(),
        organization: optional(v.get("organization")),
        period: optional(v.get("period")),
        summary: optional(v.get("summary")),
        achievements: strings(v.get("achievements")),
        skills: strings(v.get("skills")),
    })
}
fn data_json(d: &ResumeRenderData) -> Value {
    json!({"header":{"fullName":d.header.full_name,"headline":d.header.headline,"email":d.header.email,"phone":d.header.phone,"location":d.header.location,"links":d.header.links},"summary":d.summary,"experience":d.experience.iter().map(entry_json).collect::<Vec<_>>(),"education":d.education.iter().map(entry_json).collect::<Vec<_>>(),"skills":d.skills,"extraSections":d.extra_sections})
}
fn parse_data(v: &Value) -> Option<ResumeRenderData> {
    let h = v.get("header")?;
    Some(ResumeRenderData {
        header: ResumeHeader {
            full_name: h.get("fullName")?.as_str()?.into(),
            headline: h
                .get("headline")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            email: optional(h.get("email")),
            phone: optional(h.get("phone")),
            location: optional(h.get("location")),
            links: strings(h.get("links")),
        },
        summary: optional(v.get("summary")),
        experience: v
            .get("experience")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(parse_entry).collect())
            .unwrap_or_default(),
        education: v
            .get("education")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(parse_entry).collect())
            .unwrap_or_default(),
        skills: strings(v.get("skills")),
        extra_sections: v
            .get("extraSections")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), strings(Some(v))))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(path: &std::path::Path) -> Connection {
        let c = Connection::open(path).unwrap();
        c.execute_batch(include_str!(
            "../../../../contracts/db/legacy_schema_v1.sql"
        ))
        .unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','Persona',1,'{}','[]',5)",[]).unwrap();
        c.execute_batch(include_str!("../db/migrations/0003_resume_versions.sql"))
            .unwrap();
        c
    }
    fn version(id: &str, parent: Option<&str>) -> ResumeVersion {
        ResumeVersion {
            id: id.into(),
            persona_id: "p".into(),
            label: id.into(),
            template: ResumeTemplate::Modern,
            revision: 0,
            data: ResumeRenderData {
                header: ResumeHeader {
                    full_name: "张三".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            parent_id: parent.map(str::to_owned),
            created_at: "2026-01-01".into(),
        }
    }
    #[test]
    fn survives_restart() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("db");
        {
            let c = setup(&p);
            SqliteResumeVersionRepository::new(&c)
                .save(&version("v1", None))
                .unwrap();
        }
        {
            let c = Connection::open(p).unwrap();
            assert_eq!(
                SqliteResumeVersionRepository::new(&c)
                    .latest("p")
                    .unwrap()
                    .unwrap()
                    .data
                    .header
                    .full_name,
                "张三"
            );
        }
    }
    #[test]
    fn keeps_five() {
        let c = setup(std::path::Path::new(":memory:"));
        let r = SqliteResumeVersionRepository::new(&c);
        for i in 0..6 {
            r.save(&version(&format!("v{i}"), None)).unwrap();
        }
        assert_eq!(r.list("p").unwrap().len(), 5);
        assert_eq!(r.latest("p").unwrap().unwrap().revision, 6);
    }
    #[test]
    fn parent_survives_for_undo() {
        let c = setup(std::path::Path::new(":memory:"));
        let r = SqliteResumeVersionRepository::new(&c);
        r.save(&version("base", None)).unwrap();
        r.save(&version("child", Some("base"))).unwrap();
        let latest = r.latest("p").unwrap().unwrap();
        assert_eq!(
            r.get(latest.parent_id.as_deref().unwrap())
                .unwrap()
                .unwrap()
                .id,
            "base"
        );
    }
}
