//! Skill graph, gap and what-if domain (CC-FR-013/014/017).
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearningResource {
    pub kind: String,
    pub title: String,
    pub source: String,
    pub url: String,
    pub estimated_hours: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub prerequisites: Vec<String>,
    pub level: u8,
    pub resources: Vec<LearningResource>,
    pub origin: SkillOrigin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkillOrigin {
    BuiltIn,
    Custom { owner_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkillError {
    Invalid(String),
    DuplicateName,
    NotFound,
    BuiltInImmutable,
    DependencyCycle,
}

impl Skill {
    pub fn validate(&self) -> Result<(), SkillError> {
        if self.id.trim().is_empty()
            || self.name.trim().is_empty()
            || self.category.trim().is_empty()
        {
            return Err(SkillError::Invalid(
                "id, name and category are required".into(),
            ));
        }
        if !(1..=3).contains(&self.level) {
            return Err(SkillError::Invalid("level must be 1..3".into()));
        }
        if self.prerequisites.iter().any(|v| v == &self.id) {
            return Err(SkillError::DependencyCycle);
        }
        let self_keys = [
            self.id.trim().to_lowercase(),
            self.name.trim().to_lowercase(),
        ];
        let aliases = self
            .aliases
            .iter()
            .map(|a| a.trim().to_lowercase())
            .collect::<BTreeSet<_>>();
        if aliases.len() != self.aliases.len()
            || aliases
                .iter()
                .any(|a| a.is_empty() || self_keys.contains(a))
        {
            return Err(SkillError::Invalid(
                "aliases must be normalized, unique, and not self-referential".into(),
            ));
        }
        if self.resources.iter().any(|v| {
            v.title.trim().is_empty()
                || v.source.trim().is_empty()
                || reqwest::Url::parse(&v.url).ok().is_none_or(|u| {
                    !matches!(u.scheme(), "http" | "https") || u.host_str().is_none()
                })
        }) {
            return Err(SkillError::Invalid("resource URL must use HTTP(S)".into()));
        }
        Ok(())
    }
}

pub fn validate_graph(skills: &[Skill], expected_builtins: usize) -> Result<(), SkillError> {
    let ids = skills
        .iter()
        .map(|v| v.id.as_str())
        .collect::<BTreeSet<_>>();
    let names = skills
        .iter()
        .map(|v| v.name.trim().to_lowercase())
        .collect::<BTreeSet<_>>();
    if ids.len() != skills.len() || names.len() != skills.len() {
        return Err(SkillError::DuplicateName);
    }
    if skills
        .iter()
        .filter(|v| v.origin == SkillOrigin::BuiltIn)
        .count()
        != expected_builtins
    {
        return Err(SkillError::Invalid(format!(
            "expected {expected_builtins} built-ins"
        )));
    }
    for skill in skills {
        skill.validate()?;
        if skill
            .prerequisites
            .iter()
            .any(|id| !ids.contains(id.as_str()))
        {
            return Err(SkillError::NotFound);
        }
    }
    let graph = skills
        .iter()
        .map(|v| {
            (
                v.id.as_str(),
                v.prerequisites
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    fn visit<'a>(
        id: &'a str,
        graph: &BTreeMap<&'a str, Vec<&'a str>>,
        active: &mut BTreeSet<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> bool {
        if active.contains(id) {
            return false;
        }
        if done.contains(id) {
            return true;
        }
        active.insert(id);
        for dependency in graph.get(id).into_iter().flatten() {
            if !visit(dependency, graph, active, done) {
                return false;
            }
        }
        active.remove(id);
        done.insert(id);
        true
    }
    let mut active = BTreeSet::new();
    let mut done = BTreeSet::new();
    if graph
        .keys()
        .any(|id| !visit(id, &graph, &mut active, &mut done))
    {
        return Err(SkillError::DependencyCycle);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillGapRow {
    pub skill_id: String,
    pub skill_name: String,
    pub required_level: u8,
    pub current_level: u8,
    pub gap: u8,
    pub evidence_count: u32,
}

/// Returns a sortable row list for accessible tables/cards; no radar-chart dataset is produced.
pub fn analyze_gaps(
    required: &[(String, String, u8)],
    current: &BTreeMap<String, (u8, u32)>,
) -> Vec<SkillGapRow> {
    let mut rows = required
        .iter()
        .map(|(id, name, required_level)| {
            let (current_level, evidence_count) = current.get(id).copied().unwrap_or((0, 0));
            SkillGapRow {
                skill_id: id.clone(),
                skill_name: name.clone(),
                required_level: *required_level,
                current_level,
                gap: required_level.saturating_sub(current_level),
                evidence_count,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.gap
            .cmp(&a.gap)
            .then_with(|| a.skill_name.cmp(&b.skill_name))
    });
    rows
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhatIfResult {
    pub baseline_score: u8,
    pub simulated_score: u8,
    pub delta: i16,
    pub added_skills: Vec<String>,
    pub remaining_missing: Vec<String>,
}

pub fn simulate_skills(
    required: &[String],
    current: &[String],
    hypothetical: &[String],
) -> WhatIfResult {
    let norm = |values: &[String]| {
        values
            .iter()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<BTreeSet<_>>()
    };
    let required = norm(required);
    let current = norm(current);
    let hypothetical = norm(hypothetical);
    let score = |available: &BTreeSet<String>| {
        if required.is_empty() {
            100
        } else {
            ((required.intersection(available).count() * 100) as f32 / required.len() as f32)
                .round() as u8
        }
    };
    let baseline_score = score(&current);
    let combined = current
        .union(&hypothetical)
        .cloned()
        .collect::<BTreeSet<_>>();
    let simulated_score = score(&combined);
    WhatIfResult {
        baseline_score,
        simulated_score,
        delta: simulated_score as i16 - baseline_score as i16,
        added_skills: hypothetical.difference(&current).cloned().collect(),
        remaining_missing: required.difference(&combined).cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaps_are_rows_sorted_by_gap_not_radar_dimensions() {
        let required = vec![("a".into(), "A".into(), 3), ("b".into(), "B".into(), 2)];
        let current = BTreeMap::from([("b".into(), (1, 2))]);
        let rows = analyze_gaps(&required, &current);
        assert_eq!(rows.iter().map(|v| v.gap).collect::<Vec<_>>(), vec![3, 1]);
    }
    #[test]
    fn what_if_does_not_mutate_and_reports_delta() {
        let result = simulate_skills(
            &["Rust".into(), "SQL".into()],
            &["Rust".into()],
            &["SQL".into()],
        );
        assert_eq!(
            (result.baseline_score, result.simulated_score, result.delta),
            (50, 100, 50)
        );
    }
}
