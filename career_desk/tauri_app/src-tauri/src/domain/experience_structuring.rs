use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredExperienceDraft {
    /// v3 contract field name is `type` (not `experienceType`).
    #[serde(rename = "type")]
    pub experience_type: String,
    pub title: String,
    pub organization: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub raw_description: String,
    pub structured_achievements: Vec<String>,
    pub skills_demonstrated: Vec<String>,
    pub industry_tags: Vec<String>,
    pub education_level: String,
    pub status: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructurePreview {
    pub draft: StructuredExperienceDraft,
    pub prompt_version: &'static str,
    pub provider: String,
    pub model: String,
    pub cache_hit: bool,
    pub warnings: Vec<String>,
}
