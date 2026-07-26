//! Resume domain model and deterministic rendering data.
//! Requirement mapping: CC-FR-006, CC-FR-007, CC-FR-018.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResumeTemplate {
    Classic,
    Modern,
    Compact,
    Executive,
    Technical,
}

impl ResumeTemplate {
    pub const ALL: [Self; 5] = [
        Self::Classic,
        Self::Modern,
        Self::Compact,
        Self::Executive,
        Self::Technical,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Modern => "modern",
            Self::Compact => "compact",
            Self::Executive => "executive",
            Self::Technical => "technical",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResumeHeader {
    pub full_name: String,
    pub headline: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub links: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResumeEntry {
    pub source_experience_id: String,
    pub title: String,
    pub organization: Option<String>,
    pub period: Option<String>,
    pub summary: Option<String>,
    pub achievements: Vec<String>,
    pub skills: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResumeRenderData {
    pub header: ResumeHeader,
    pub summary: Option<String>,
    pub experience: Vec<ResumeEntry>,
    pub education: Vec<ResumeEntry>,
    pub skills: Vec<String>,
    pub extra_sections: BTreeMap<String, Vec<String>>,
}

impl ResumeRenderData {
    pub fn normalize(&mut self) {
        self.header.links = deduplicate(&self.header.links);
        self.skills = deduplicate(&self.skills);
        for entry in self.experience.iter_mut().chain(self.education.iter_mut()) {
            entry.achievements = deduplicate(&entry.achievements);
            entry.skills = deduplicate(&entry.skills);
        }
    }

    pub fn validate(&self) -> Result<(), ResumeError> {
        if self.header.full_name.trim().is_empty() {
            return Err(ResumeError::Validation("full name is required".into()));
        }
        if self
            .experience
            .iter()
            .any(|entry| entry.title.trim().is_empty())
        {
            return Err(ResumeError::Validation(
                "experience title is required".into(),
            ));
        }
        Ok(())
    }
}

fn deduplicate(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.to_lowercase()))
        .map(str::to_owned)
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeVersion {
    pub id: String,
    pub persona_id: String,
    pub label: String,
    pub template: ResumeTemplate,
    pub revision: u32,
    pub data: ResumeRenderData,
    pub parent_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeEdit {
    SetSummary {
        before: Option<String>,
        after: Option<String>,
    },
    SetHeadline {
        before: String,
        after: String,
    },
    ReplaceEntry {
        index: usize,
        before: Box<ResumeEntry>,
        after: Box<ResumeEntry>,
    },
}

impl ResumeEdit {
    pub fn apply(&self, data: &mut ResumeRenderData) -> Result<(), ResumeError> {
        match self {
            Self::SetSummary { after, .. } => data.summary = after.clone(),
            Self::SetHeadline { after, .. } => data.header.headline = after.clone(),
            Self::ReplaceEntry { index, after, .. } => {
                let target = data
                    .experience
                    .get_mut(*index)
                    .ok_or(ResumeError::InvalidEdit)?;
                *target = after.as_ref().clone();
            }
        }
        Ok(())
    }

    pub fn undo(&self, data: &mut ResumeRenderData) -> Result<(), ResumeError> {
        match self {
            Self::SetSummary { before, .. } => data.summary = before.clone(),
            Self::SetHeadline { before, .. } => data.header.headline = before.clone(),
            Self::ReplaceEntry { index, before, .. } => {
                let target = data
                    .experience
                    .get_mut(*index)
                    .ok_or(ResumeError::InvalidEdit)?;
                *target = before.as_ref().clone();
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResumeDiff {
    pub header_changed: bool,
    pub summary_changed: bool,
    pub experience_added: Vec<String>,
    pub experience_removed: Vec<String>,
    pub experience_changed: Vec<String>,
    pub skills_added: Vec<String>,
    pub skills_removed: Vec<String>,
}

pub fn diff(left: &ResumeVersion, right: &ResumeVersion) -> ResumeDiff {
    let left_entries: BTreeMap<_, _> = left
        .data
        .experience
        .iter()
        .map(|entry| (entry.source_experience_id.clone(), entry))
        .collect();
    let right_entries: BTreeMap<_, _> = right
        .data
        .experience
        .iter()
        .map(|entry| (entry.source_experience_id.clone(), entry))
        .collect();
    let left_skills: BTreeSet<_> = left.data.skills.iter().cloned().collect();
    let right_skills: BTreeSet<_> = right.data.skills.iter().cloned().collect();
    ResumeDiff {
        header_changed: left.data.header != right.data.header,
        summary_changed: left.data.summary != right.data.summary,
        experience_added: right_entries
            .keys()
            .filter(|id| !left_entries.contains_key(*id))
            .cloned()
            .collect(),
        experience_removed: left_entries
            .keys()
            .filter(|id| !right_entries.contains_key(*id))
            .cloned()
            .collect(),
        experience_changed: left_entries
            .iter()
            .filter_map(|(id, entry)| {
                right_entries
                    .get(id)
                    .filter(|other| *other != entry)
                    .map(|_| id.clone())
            })
            .collect(),
        skills_added: right_skills.difference(&left_skills).cloned().collect(),
        skills_removed: left_skills.difference(&right_skills).cloned().collect(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeError {
    Validation(String),
    InvalidEdit,
    VersionLimit,
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> ResumeRenderData {
        ResumeRenderData {
            header: ResumeHeader {
                full_name: "Ada Lovelace".into(),
                headline: "Engineer".into(),
                ..Default::default()
            },
            skills: vec!["Rust".into(), "rust".into(), "  SQL ".into()],
            ..Default::default()
        }
    }

    #[test]
    fn exposes_exactly_five_stable_templates() {
        assert_eq!(
            ResumeTemplate::ALL.map(ResumeTemplate::id),
            ["classic", "modern", "compact", "executive", "technical"]
        );
    }

    #[test]
    fn edit_can_be_applied_and_undone_without_loss() {
        let mut resume = data();
        let original = resume.clone();
        let edit = ResumeEdit::SetHeadline {
            before: "Engineer".into(),
            after: "Staff Engineer".into(),
        };
        edit.apply(&mut resume).unwrap();
        edit.undo(&mut resume).unwrap();
        assert_eq!(resume, original);
    }

    #[test]
    fn diff_reports_ab_changes_by_stable_experience_id() {
        let mut left_data = data();
        left_data.experience.push(ResumeEntry {
            source_experience_id: "exp-1".into(),
            title: "Developer".into(),
            ..Default::default()
        });
        let mut right_data = left_data.clone();
        right_data.experience[0].title = "Senior Developer".into();
        right_data.skills.push("Tauri".into());
        let version = |id: &str, data| ResumeVersion {
            id: id.into(),
            persona_id: "p1".into(),
            label: id.into(),
            template: ResumeTemplate::Classic,
            revision: 1,
            data,
            parent_id: None,
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let result = diff(&version("a", left_data), &version("b", right_data));
        assert_eq!(result.experience_changed, vec!["exp-1"]);
        assert_eq!(result.skills_added, vec!["Tauri"]);
    }

    #[test]
    fn normalization_is_stable_and_case_insensitive() {
        let mut resume = data();
        resume.normalize();
        assert_eq!(resume.skills, vec!["Rust", "SQL"]);
    }
}
