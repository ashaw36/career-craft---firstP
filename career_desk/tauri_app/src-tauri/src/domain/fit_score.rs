//! Deterministic Fit Score baseline equivalent to Python PersonaEngine.
//! CC-FR-005 / FIT-001..006.

use super::entities::{Experience, Persona};

pub fn calculate(persona: &Persona, experience: &Experience) -> f64 {
    let total: f64 = persona.capability_weights.iter().map(|(_, w)| *w).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let keywords = keywords(experience);
    let matched: f64 = persona
        .capability_weights
        .iter()
        .filter(|(skill, _)| keyword_match(skill, &keywords))
        .map(|(_, weight)| *weight)
        .sum();
    (matched / total).clamp(0.0, 1.0)
}

pub fn apply_override(score: f64) -> f64 {
    score.clamp(0.0, 1.0)
}

fn keywords(exp: &Experience) -> Vec<String> {
    let mut values = exp.skills_demonstrated.clone();
    values.push(exp.title.clone());
    if let Some(org) = &exp.organization {
        values.push(org.clone());
    }
    values.extend(exp.structured_achievements.iter().flat_map(|s| tokenize(s)));
    values.extend(tokenize(&exp.raw_description));
    values
        .into_iter()
        .filter_map(|v| {
            let v = v.trim().to_lowercase();
            (!v.is_empty()).then_some(v)
        })
        .collect()
}

fn tokenize(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut ascii = String::new();
    let mut chinese = String::new();
    let flush = |buf: &mut String, out: &mut Vec<String>| {
        if !buf.is_empty() {
            out.push(std::mem::take(buf));
        }
    };
    for c in value.chars() {
        if c.is_ascii_alphabetic() {
            flush(&mut chinese, &mut result);
            ascii.push(c);
        } else if ('\u{4e00}'..='\u{9fff}').contains(&c) {
            flush(&mut ascii, &mut result);
            chinese.push(c);
        } else {
            flush(&mut ascii, &mut result);
            flush(&mut chinese, &mut result);
        }
    }
    flush(&mut ascii, &mut result);
    flush(&mut chinese, &mut result);
    result
}

fn keyword_match(skill: &str, keywords: &[String]) -> bool {
    let skill = skill.trim().to_lowercase();
    !skill.is_empty()
        && keywords
            .iter()
            .any(|kw| skill.contains(kw) || kw.contains(&skill))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ExperienceStatus, ExperienceType};
    fn exp() -> Experience {
        Experience {
            id: "e".into(),
            user_id: "default".into(),
            kind: ExperienceType::Work,
            title: "高级后端工程师".into(),
            organization: None,
            start_date: None,
            end_date: None,
            raw_description: "使用 Rust 构建服务".into(),
            structured_achievements: vec![],
            skills_demonstrated: vec!["Python".into()],
            industry_tags: vec![],
            education_level: None,
            status: ExperienceStatus::Confirmed,
            version: 1,
        }
    }
    fn persona() -> Persona {
        Persona {
            id: "p".into(),
            user_id: "default".into(),
            name: "技术".into(),
            is_default: true,
            identity_statement: None,
            career_narrative: None,
            tone_style: None,
            capability_weights: vec![("Python".into(), 0.6), ("Rust".into(), 0.4)],
            target_job_profiles: vec![],
            max_experiences: 5,
            preferred_model: None,
        }
    }
    #[test]
    fn fit_matches_legacy_weight_ratio() {
        assert_eq!(calculate(&persona(), &exp()), 1.0);
    }
    #[test]
    fn empty_weights_are_zero() {
        let mut p = persona();
        p.capability_weights.clear();
        assert_eq!(calculate(&p, &exp()), 0.0);
    }
    #[test]
    fn override_is_clamped() {
        assert_eq!(apply_override(2.0), 1.0);
        assert_eq!(apply_override(-1.0), 0.0);
    }
}
