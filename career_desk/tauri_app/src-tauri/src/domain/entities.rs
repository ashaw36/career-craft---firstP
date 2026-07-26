//! Legacy-compatible domain records. Persistence DTOs must remain outside this module.
//! Requirement mapping: CC-FR-001..018.

#[derive(Clone, Debug, PartialEq)]
pub struct Experience {
    pub id: String,
    pub user_id: String,
    pub kind: ExperienceType,
    pub title: String,
    pub organization: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// Immutable user input; AI output must never overwrite it (CC-FR-001/CC-NFR-005).
    pub raw_description: String,
    pub structured_achievements: Vec<String>,
    pub skills_demonstrated: Vec<String>,
    pub industry_tags: Vec<String>,
    pub education_level: Option<EducationLevel>,
    pub status: ExperienceStatus,
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EducationLevel {
    HighSchool,
    Associate,
    Bachelor,
    Master,
    Doctorate,
    Other,
}
impl EducationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HighSchool => "high_school",
            Self::Associate => "associate",
            Self::Bachelor => "bachelor",
            Self::Master => "master",
            Self::Doctorate => "doctorate",
            Self::Other => "other",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "high_school" | "high school" | "高中" => Some(Self::HighSchool),
            "associate" | "大专" | "专科" => Some(Self::Associate),
            "bachelor" | "bachelors" | "本科" | "学士" => Some(Self::Bachelor),
            "master" | "masters" | "硕士" => Some(Self::Master),
            "doctorate" | "doctoral" | "phd" | "博士" => Some(Self::Doctorate),
            "other" | "其他" => Some(Self::Other),
            "none" | "" => None,
            _ => None,
        }
    }
    pub fn rank(&self) -> u8 {
        match self {
            Self::HighSchool => 1,
            Self::Associate => 2,
            Self::Bachelor => 3,
            Self::Master => 4,
            Self::Doctorate => 5,
            Self::Other => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExperienceType {
    Work,
    Project,
    Education,
    Certification,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExperienceStatus {
    Draft,
    Confirmed,
    Discarded,
    Archived,
}

/// User-editable fields for explicit user edits.
/// AI enrichment must use `ExperienceEnrichment` and never overwrite `raw_description` (CC-FR-001).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExperiencePatch {
    pub title: Option<String>,
    pub organization: Option<Option<String>>,
    pub start_date: Option<Option<String>>,
    pub end_date: Option<Option<String>>,
    pub status: Option<ExperienceStatus>,
    pub industry_tags: Option<Vec<String>>,
    pub education_level: Option<Option<EducationLevel>>,
    pub raw_description: Option<String>,
    pub kind: Option<ExperienceType>,
    pub structured_achievements: Option<Vec<String>>,
    pub skills_demonstrated: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExperienceEnrichment {
    pub structured_achievements: Vec<String>,
    pub skills_demonstrated: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Persona {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub is_default: bool,
    pub identity_statement: Option<String>,
    pub career_narrative: Option<String>,
    pub tone_style: Option<String>,
    pub capability_weights: Vec<(String, f64)>,
    pub target_job_profiles: Vec<String>,
    pub max_experiences: u32,
    pub preferred_model: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PersonaPatch {
    pub name: Option<String>,
    pub identity_statement: Option<Option<String>>,
    pub career_narrative: Option<Option<String>>,
    pub tone_style: Option<Option<String>>,
    pub capability_weights: Option<Vec<(String, f64)>>,
    pub target_job_profiles: Option<Vec<String>>,
    pub max_experiences: Option<u32>,
    pub preferred_model: Option<Option<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoleExperienceWeight {
    pub id: String,
    pub persona_id: String,
    pub experience_id: String,
    pub relevance_score: f64,
    pub reframed_summary: Option<String>,
    pub highlighted_skills: Vec<String>,
    pub user_overridden: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JobDescription {
    pub id: String,
    pub raw_text: String,
    pub title: Option<String>,
    pub company: Option<String>,
    pub years_of_experience: Option<String>,
    pub parsed_skills: Vec<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JobMatch {
    pub id: String,
    pub persona_id: String,
    pub job_desc_id: String,
    pub match_score: u8,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub tracking_status: TrackingStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrackingStatus {
    New,
    Interested,
    Applied,
    Interviewing,
    Offered,
    Rejected,
    Ghosted,
    Accepted,
    Declined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillNode {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LearningPath {
    pub id: String,
    pub persona_id: String,
    pub target_gap: Option<String>,
    pub source_type: Option<String>,
    pub status: String,
}
