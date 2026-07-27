//! Job analysis domain model (CC-FR-009..012/020).
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedJob {
    pub title: Option<String>,
    pub company: Option<String>,
    pub required_skills: Vec<String>,
    pub preferred_skills: Vec<String>,
    pub minimum_years: Option<f32>,
    pub industry_terms: Vec<String>,
    pub education_terms: Vec<String>,
    pub raw_text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandidateEvidence {
    pub skills: Vec<String>,
    pub years_experience: f32,
    pub industry_terms: Vec<String>,
    pub education_terms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchBreakdown {
    pub skill_score: f32,
    pub experience_score: f32,
    pub industry_score: f32,
    pub education_score: f32,
    pub total: u8,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
}

fn normalized(items: &[String]) -> BTreeSet<String> {
    items
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}

fn overlap_ratio(expected: &[String], actual: &[String]) -> f32 {
    let expected = normalized(expected);
    if expected.is_empty() {
        return 1.0;
    }
    let actual = normalized(actual);
    expected.intersection(&actual).count() as f32 / expected.len() as f32
}

pub fn skill_match(required: &[String], actual: &[String]) -> (f32, Vec<String>, Vec<String>) {
    let required = normalized(required);
    let actual = normalized(actual);
    let matched = required.intersection(&actual).cloned().collect::<Vec<_>>();
    let missing = required.difference(&actual).cloned().collect::<Vec<_>>();
    let ratio = if required.is_empty() { 1.0 } else { matched.len() as f32 / required.len() as f32 };
    (ratio * 50.0, matched, missing)
}

/// Stable, deterministic 50/25/15/10 scoring required by CC-FR-010.
pub fn score_match(job: &ParsedJob, candidate: &CandidateEvidence) -> MatchBreakdown {
    let (skill_score, matched_skills, missing_skills) = skill_match(&job.required_skills, &candidate.skills);
    let experience_ratio = match job.minimum_years {
        None | Some(0.0) => 1.0,
        Some(years) => (candidate.years_experience / years).clamp(0.0, 1.0),
    };
    let experience_score = experience_ratio * 25.0;
    let industry_score = overlap_ratio(&job.industry_terms, &candidate.industry_terms) * 15.0;
    let education_score = education_ratio(&job.education_terms, &candidate.education_terms) * 10.0;
    let total = (skill_score + experience_score + industry_score + education_score)
        .round()
        .clamp(0.0, 100.0) as u8;
    MatchBreakdown {
        skill_score,
        experience_score,
        industry_score,
        education_score,
        total,
        matched_skills,
        missing_skills,
    }
}
fn education_rank(value: &str) -> u8 {
    match value.trim().to_lowercase().as_str() {
        "high_school" | "high school" | "高中" => 1,
        "associate" | "大专" | "专科" => 2,
        "bachelor" | "本科" | "学士" | "degree" => 3,
        "master" | "硕士" => 4,
        "doctorate" | "phd" | "博士" => 5,
        _ => 0,
    }
}
fn education_ratio(expected: &[String], actual: &[String]) -> f32 {
    let required = expected
        .iter()
        .map(|v| education_rank(v))
        .max()
        .unwrap_or(0);
    if required == 0 {
        return 1.0;
    }
    let candidate = actual.iter().map(|v| education_rank(v)).max().unwrap_or(0);
    if candidate >= required {
        1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
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

impl JobStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        use JobStatus::*;
        self == next
            || matches!(
                (self, next),
                (New, Interested | Applied | Rejected | Declined)
                    | (Interested, Applied | Rejected | Declined)
                    | (Applied, Interviewing | Rejected | Ghosted | Declined)
                    | (Interviewing, Offered | Rejected | Ghosted | Declined)
                    | (Offered, Accepted | Declined)
                    | (Ghosted, Interviewing | Rejected | Declined)
            )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReframeSuggestion {
    pub id: String,
    pub match_id: String,
    pub experience_id: String,
    pub original_text: String,
    pub suggested_text: String,
    pub edited_text: Option<String>,
}

impl ReframeSuggestion {
    pub fn effective_text(&self) -> &str {
        self.edited_text.as_deref().unwrap_or(&self.suggested_text)
    }
    pub fn update(&mut self, text: String) {
        self.edited_text = Some(text);
    }
    pub fn reset(&mut self) {
        self.edited_text = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_uses_required_weights() {
        let job = ParsedJob {
            required_skills: vec!["Rust".into(), "SQL".into()],
            minimum_years: Some(4.0),
            industry_terms: vec!["fintech".into()],
            education_terms: vec!["bachelor".into()],
            ..Default::default()
        };
        let candidate = CandidateEvidence {
            skills: vec!["rust".into()],
            years_experience: 2.0,
            industry_terms: vec!["FinTech".into()],
            education_terms: vec![],
        };
        let result = score_match(&job, &candidate);
        assert_eq!(result.skill_score, 25.0);
        assert_eq!(result.experience_score, 12.5);
        assert_eq!(result.industry_score, 15.0);
        assert_eq!(result.education_score, 0.0);
        assert_eq!(result.total, 53);
    }
    #[test]
    fn master_meets_bachelor_requirement() {
        let job = ParsedJob {
            education_terms: vec!["bachelor".into()],
            ..Default::default()
        };
        let candidate = CandidateEvidence {
            education_terms: vec!["master".into()],
            ..Default::default()
        };
        assert_eq!(score_match(&job, &candidate).education_score, 10.0)
    }
    #[test]
    fn bachelor_does_not_meet_master_requirement() {
        let job = ParsedJob {
            education_terms: vec!["master".into()],
            ..Default::default()
        };
        let candidate = CandidateEvidence {
            education_terms: vec!["bachelor".into()],
            ..Default::default()
        };
        assert_eq!(score_match(&job, &candidate).education_score, 0.0)
    }

    #[test]
    fn what_if_skill_component_uses_the_same_fifty_point_scale() {
        let required = vec!["Rust".into(), "SQL".into()];
        let (before, _, missing) = skill_match(&required, &["Rust".into()]);
        let (after, matched, remaining) = skill_match(&required, &["Rust".into(), "SQL".into()]);
        assert_eq!((before, after), (25.0, 50.0));
        assert_eq!(missing, vec!["sql"]);
        assert_eq!(matched, vec!["rust", "sql"]);
        assert!(remaining.is_empty());
    }

    #[test]
    fn terminal_status_cannot_regress() {
        assert!(!JobStatus::Accepted.can_transition_to(JobStatus::Applied));
        assert!(JobStatus::Offered.can_transition_to(JobStatus::Accepted));
    }

    #[test]
    fn reframe_reset_restores_suggestion() {
        let mut item = ReframeSuggestion {
            id: "r".into(),
            match_id: "m".into(),
            experience_id: "e".into(),
            original_text: "old".into(),
            suggested_text: "new".into(),
            edited_text: None,
        };
        item.update("edited".into());
        assert_eq!(item.effective_text(), "edited");
        item.reset();
        assert_eq!(item.effective_text(), "new");
    }
}
