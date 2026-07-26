//! Complete built-in catalog parser (CC-FR-014). The versioned JSON is bundled.
use crate::domain::skills::{LearningResource, Skill, SkillError, SkillOrigin};
use serde::Deserialize;
use std::collections::BTreeMap;

const CATALOG: &str = include_str!("../../../assets/skill_graph_v1.json");
#[derive(Deserialize)]
struct Row {
    id: String,
    name: String,
    category: String,
    description: String,
    aliases: Vec<String>,
    prerequisites: Vec<String>,
    level: u8,
    resources: Vec<Resource>,
}
#[derive(Deserialize)]
struct Resource {
    #[serde(rename = "type")]
    kind: String,
    title: String,
    source: String,
    url: String,
    estimated_hours: u16,
}
fn normalized_resources(rows: Vec<Resource>) -> Result<Vec<LearningResource>, SkillError> {
    let mut unique = BTreeMap::new();
    for row in rows {
        let mut url = reqwest::Url::parse(row.url.trim())
            .map_err(|_| SkillError::Invalid("resource URL is invalid".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(SkillError::Invalid("resource URL must use HTTP(S)".into()));
        }
        url.set_fragment(None);
        let normalized = url.to_string().trim_end_matches('/').to_owned();
        unique
            .entry(normalized.clone())
            .or_insert(LearningResource {
                kind: row.kind.trim().to_lowercase(),
                title: row.title.trim().to_owned(),
                source: row.source.trim().to_owned(),
                url: normalized,
                estimated_hours: row.estimated_hours,
            });
    }
    Ok(unique.into_values().collect())
}

pub fn builtin_skills() -> Result<Vec<Skill>, SkillError> {
    let rows: Vec<Row> = serde_json::from_str(CATALOG)
        .map_err(|e| SkillError::Invalid(format!("invalid skill catalog: {e}")))?;
    let values = rows
        .into_iter()
        .map(|v| {
            let mut aliases = v
                .aliases
                .into_iter()
                .map(|a| a.trim().to_lowercase())
                .filter(|a| !a.is_empty())
                .collect::<Vec<_>>();
            aliases.sort();
            aliases.dedup();
            Ok(Skill {
                id: v.id,
                name: v.name,
                category: v.category,
                description: v.description,
                aliases,
                prerequisites: v
                    .prerequisites
                    .into_iter()
                    .map(|p| p.trim().to_lowercase())
                    .collect(),
                level: v.level,
                resources: normalized_resources(v.resources)?,
                origin: SkillOrigin::BuiltIn,
            })
        })
        .collect::<Result<Vec<_>, SkillError>>()?;
    crate::domain::skills::validate_graph(&values, 51)?;
    if values.iter().any(|skill| skill.resources.len() < 3) {
        return Err(SkillError::Invalid(
            "trusted catalog must provide at least three resources per skill".into(),
        ));
    }
    Ok(values)
}

/// Returns true only when the normalized URL belongs to the bundled learning catalog.
/// This keeps resource opening independent from the job-page collector while preserving
/// the single-use external-open token boundary.
pub fn is_builtin_resource_url(candidate: &str) -> bool {
    let Ok(mut url) = reqwest::Url::parse(candidate.trim()) else {
        return false;
    };
    if url.scheme() != "https" || url.host_str().is_none() {
        return false;
    }
    url.set_fragment(None);
    let normalized = url.to_string().trim_end_matches('/').to_owned();
    builtin_skills().is_ok_and(|skills| {
        skills
            .iter()
            .flat_map(|skill| &skill.resources)
            .any(|resource| resource.url == normalized)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frozen_catalog_has_exactly_51_valid_nodes() {
        let skills = builtin_skills().unwrap();
        assert_eq!(skills.len(), 51);
        assert!(skills.iter().any(|v| v.id == "llm_applications"));
        assert!(skills
            .iter()
            .all(|v| !v.description.is_empty() && !v.resources.is_empty()));
        assert!(skills
            .iter()
            .flat_map(|v| &v.resources)
            .all(|r| r.url.starts_with("http")));
    }
    #[test]
    fn resource_urls_are_normalized_and_deduplicated() {
        let rows = vec![
            Resource {
                kind: "DOC".into(),
                title: " A ".into(),
                source: " S ".into(),
                url: "https://example.com/docs/#part".into(),
                estimated_hours: 1,
            },
            Resource {
                kind: "doc".into(),
                title: "B".into(),
                source: "S".into(),
                url: "https://example.com/docs".into(),
                estimated_hours: 2,
            },
        ];
        let values = normalized_resources(rows).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].url, "https://example.com/docs");
        assert_eq!(values[0].kind, "doc")
    }
    #[test]
    fn trusted_resource_lookup_accepts_only_bundled_https_urls() {
        assert!(is_builtin_resource_url(
            "https://www.nngroup.com/articles/user-research-methods/#section"
        ));
        assert!(!is_builtin_resource_url(
            "https://example.com/not-in-catalog"
        ));
        assert!(!is_builtin_resource_url(
            "http://www.nngroup.com/articles/user-research-methods/"
        ));
        assert!(!is_builtin_resource_url("javascript:alert(1)"));
    }
}
