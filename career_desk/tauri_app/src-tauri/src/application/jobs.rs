//! JD parsing, job workflow and URL fallback use cases (CC-FR-009..012/020).
use crate::domain::{
    entities::{Experience, ExperienceStatus, ExperienceType, Persona},
    jobs::{CandidateEvidence, JobStatus, ParsedJob},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobWorkflowError {
    InvalidTransition,
    EmptyDescription,
}

pub fn transition(current: JobStatus, next: JobStatus) -> Result<JobStatus, JobWorkflowError> {
    current
        .can_transition_to(next)
        .then_some(next)
        .ok_or(JobWorkflowError::InvalidTransition)
}

/// Deterministic local parser used immediately and as a fallback when LLM parsing is unavailable.
/// It deliberately preserves `raw_text`; an LLM adapter may enrich but never replace user input.
pub fn parse_jd_locally(text: &str) -> Result<ParsedJob, JobWorkflowError> {
    let raw = text.trim();
    if raw.is_empty() {
        return Err(JobWorkflowError::EmptyDescription);
    }
    let lower = raw.to_lowercase();
    let known_skills = [
        "rust",
        "python",
        "java",
        "javascript",
        "typescript",
        "sql",
        "aws",
        "azure",
        "docker",
        "kubernetes",
        "react",
        "vue",
        "tauri",
    ];
    let required_skills = known_skills
        .iter()
        .filter(|skill| lower.contains(**skill))
        .map(|v| (*v).to_string())
        .collect();
    let minimum_years = extract_years(&lower);
    let mut lines = raw.lines().map(str::trim).filter(|line| !line.is_empty());
    let title = lines.next().map(str::to_owned);
    let industry_terms = detected_terms(&lower, INDUSTRY_TERMS);
    let education_terms = detected_terms(&lower, EDUCATION_TERMS);
    Ok(ParsedJob {
        title,
        required_skills,
        minimum_years,
        industry_terms,
        education_terms,
        raw_text: raw.to_owned(),
        ..Default::default()
    })
}

const INDUSTRY_TERMS: &[&str] = &[
    "fintech",
    "finance",
    "banking",
    "healthcare",
    "ecommerce",
    "retail",
    "manufacturing",
    "automotive",
    "金融",
    "银行",
    "医疗",
    "电商",
    "零售",
    "制造",
    "汽车",
];
const EDUCATION_TERMS: &[&str] = &[
    "bachelor", "master", "phd", "degree", "本科", "学士", "硕士", "博士", "学历",
];

fn detected_terms(text: &str, terms: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for term in terms.iter().filter(|term| text.contains(**term)) {
        let value = canonical_term(term);
        if !out.iter().any(|v: &String| v == value) {
            out.push(value.into())
        }
    }
    out
}
fn canonical_term(value: &str) -> &str {
    match value {
        "金融科技" => "fintech",
        "金融" => "finance",
        "银行" => "banking",
        "医疗" => "healthcare",
        "电商" => "ecommerce",
        "零售" => "retail",
        "制造" => "manufacturing",
        "汽车" => "automotive",
        "高中" => "high_school",
        "大专" | "专科" => "associate",
        "本科" | "学士" | "degree" => "bachelor",
        "硕士" => "master",
        "博士" | "phd" => "doctorate",
        other => other,
    }
}

/// Builds deterministic candidate evidence from confirmed persona experiences.
/// Industry/education are deliberately conservative keyword heuristics until structured fields exist.
pub fn candidate_evidence(persona: &Persona, experiences: &[Experience]) -> CandidateEvidence {
    candidate_evidence_at(persona, experiences, current_month_index())
}

fn candidate_evidence_at(
    persona: &Persona,
    experiences: &[Experience],
    current_month: i32,
) -> CandidateEvidence {
    let confirmed = experiences
        .iter()
        .filter(|e| e.status == ExperienceStatus::Confirmed)
        .collect::<Vec<_>>();
    let mut skills = persona
        .capability_weights
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    for value in &confirmed {
        skills.extend(value.skills_demonstrated.clone());
    }
    let searchable = confirmed
        .iter()
        .map(|e| {
            format!(
                "{} {} {} {}",
                e.title,
                e.organization.as_deref().unwrap_or(""),
                e.raw_description,
                e.structured_achievements.join(" ")
            )
            .to_lowercase()
        })
        .collect::<Vec<_>>()
        .join(" ");
    let education_text = confirmed
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                ExperienceType::Education | ExperienceType::Certification
            )
        })
        .map(|e| format!("{} {}", e.title, e.raw_description).to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let persisted_industry = confirmed
        .iter()
        .flat_map(|e| e.industry_tags.iter().cloned())
        .collect::<Vec<_>>();
    let persisted_education = confirmed
        .iter()
        .filter_map(|e| e.education_level.as_ref().map(|v| v.as_str().to_owned()))
        .collect::<Vec<_>>();
    CandidateEvidence {
        skills,
        years_experience: merged_work_months(&confirmed, current_month) as f32 / 12.0,
        industry_terms: if persisted_industry.is_empty() {
            detected_terms(&searchable, INDUSTRY_TERMS)
        } else {
            persisted_industry
                .into_iter()
                .map(|v| canonical_term(&v).to_owned())
                .collect()
        },
        education_terms: if persisted_education.is_empty() {
            detected_terms(&education_text, EDUCATION_TERMS)
        } else {
            persisted_education
        },
    }
}

fn month_index(value: &str) -> Option<i32> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts
        .next()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    (1..=12).contains(&month).then_some(year * 12 + month - 1)
}
fn current_month_index() -> i32 {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| (v.as_secs() / 86_400) as i64)
        .unwrap_or(0);
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1
    };
    year as i32 * 12 + month as i32 - 1
}
fn merged_work_months(values: &[&Experience], current_month: i32) -> u32 {
    let mut ranges = values
        .iter()
        .filter(|e| matches!(e.kind, ExperienceType::Work | ExperienceType::Project))
        .filter_map(|e| {
            Some((
                month_index(e.start_date.as_deref()?)?,
                match e.end_date.as_deref() {
                    Some(v) => month_index(v)?,
                    None => current_month,
                },
            ))
        })
        .filter(|(start, end)| end >= start)
        .collect::<Vec<_>>();
    ranges.sort_unstable();
    let mut total = 0;
    let mut merged: Option<(i32, i32)> = None;
    for (start, end) in ranges {
        match merged {
            Some((s, e)) if start <= e + 1 => merged = Some((s, e.max(end))),
            Some((s, e)) => {
                total += (e - s + 1) as u32;
                merged = Some((start, end))
            }
            None => merged = Some((start, end)),
        }
    }
    if let Some((s, e)) = merged {
        total += (e - s + 1) as u32
    }
    total
}

fn extract_years(value: &str) -> Option<f32> {
    for token in value.split(|c: char| !(c.is_ascii_digit() || c == '.')) {
        if token.is_empty() {
            continue;
        }
        if let Ok(years) = token.parse::<f32>() {
            if (0.0..=50.0).contains(&years) {
                return Some(years);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_parser_is_available_without_network() {
        let parsed =
            parse_jd_locally("Senior Rust Engineer\nRequires 5 years, SQL and Docker").unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Senior Rust Engineer"));
        assert_eq!(parsed.minimum_years, Some(5.0));
        assert_eq!(parsed.required_skills, vec!["rust", "sql", "docker"]);
    }
    #[test]
    fn rejects_invalid_status_regression() {
        assert_eq!(
            transition(JobStatus::Accepted, JobStatus::New),
            Err(JobWorkflowError::InvalidTransition)
        );
    }
    fn persona() -> Persona {
        Persona {
            id: "p".into(),
            user_id: "u".into(),
            name: "P".into(),
            is_default: false,
            identity_statement: None,
            career_narrative: None,
            tone_style: None,
            capability_weights: vec![("Rust".into(), 1.0)],
            target_job_profiles: vec![],
            max_experiences: 3,
            preferred_model: None,
        }
    }
    fn experience(id: &str, start: &str, end: &str) -> Experience {
        Experience {
            id: id.into(),
            user_id: "u".into(),
            kind: ExperienceType::Work,
            title: "Fintech Engineer".into(),
            organization: Some("Bank".into()),
            start_date: Some(start.into()),
            end_date: Some(end.into()),
            raw_description: "Finance platform".into(),
            structured_achievements: vec![],
            skills_demonstrated: vec!["SQL".into()],
            industry_tags: vec![],
            education_level: None,
            status: ExperienceStatus::Confirmed,
            version: 1,
        }
    }
    #[test]
    fn candidate_uses_skill_industry_education_and_merged_year_evidence() {
        let a = experience("a", "2020-01", "2022-12");
        let b = experience("b", "2022-01", "2023-12");
        let mut edu = experience("edu", "2016-01", "2020-01");
        edu.kind = ExperienceType::Education;
        edu.title = "Bachelor degree".into();
        let evidence = candidate_evidence(&persona(), &[a, b, edu]);
        assert!(
            evidence.skills.contains(&"Rust".into()) && evidence.skills.contains(&"SQL".into())
        );
        assert!(evidence.industry_terms.contains(&"fintech".into()));
        assert!(evidence.education_terms.contains(&"bachelor".into()));
        assert!((evidence.years_experience - 4.0).abs() < 0.01)
    }
    #[test]
    fn parser_exposes_conservative_job_terms() {
        let parsed =
            parse_jd_locally("Fintech Rust Engineer requires bachelor degree and 4 years").unwrap();
        assert_eq!(parsed.industry_terms, vec!["fintech"]);
        assert!(parsed.education_terms.contains(&"bachelor".into()))
    }
    #[test]
    fn ongoing_range_uses_injected_current_month_at_year_boundary() {
        let ongoing = experience("ongoing", "2025-12", "");
        let mut ongoing = ongoing;
        ongoing.end_date = None;
        let evidence = candidate_evidence_at(&persona(), &[ongoing], 2026 * 12);
        assert!((evidence.years_experience - (2.0 / 12.0)).abs() < 0.001);
        assert_eq!(month_index("2026-01"), Some(2026 * 12));
    }
}
