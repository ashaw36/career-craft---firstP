pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "legacy_baseline",
        sql: include_str!("migrations/0001_legacy_baseline.sql"),
    },
    Migration {
        version: 2,
        name: "provider_resume",
        sql: include_str!("migrations/0002_provider_resume.sql"),
    },
    Migration {
        version: 3,
        name: "resume_versions",
        sql: include_str!("migrations/0003_resume_versions.sql"),
    },
    Migration {
        version: 4,
        name: "skills_learning_tasks",
        sql: include_str!("migrations/0004_skills_learning_tasks.sql"),
    },
    Migration {
        version: 5,
        name: "structured_evidence",
        sql: include_str!("migrations/0005_structured_evidence.sql"),
    },
    Migration {
        version: 6,
        name: "llm_cache",
        sql: include_str!("migrations/0006_llm_cache.sql"),
    },
    Migration {
        version: 7,
        name: "jobs_learning_audit",
        sql: include_str!("migrations/0007_jobs_learning_audit.sql"),
    },
    Migration {
        version: 8,
        name: "versions_conversion_snapshot",
        sql: include_str!("migrations/0008_versions_conversion_snapshot.sql"),
    },
    Migration {
        version: 9,
        name: "experience_revision_history",
        sql: include_str!("migrations/0009_experience_revision_history.sql"),
    },
    Migration {
        version: 10,
        name: "experience_revisions_append_only",
        sql: include_str!("migrations/0010_experience_revisions_append_only.sql"),
    },
];
