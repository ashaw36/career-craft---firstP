//! CC-FR-005 application use-case, independent of transport and persistence.
use super::ports::{ApplicationError, ExperienceRepository, PersonaRepository};
use crate::domain::{entities::RoleExperienceWeight, fit_score};

pub fn recalculate<E: ExperienceRepository, P: PersonaRepository>(
    experiences: &E,
    personas: &P,
    persona_id: &str,
) -> Result<Vec<RoleExperienceWeight>, ApplicationError> {
    let persona = personas
        .get(persona_id)?
        .ok_or_else(|| ApplicationError::NotFound("persona".into()))?;
    let existing = personas.get_weights(persona_id)?;
    let rows = experiences
        .list_confirmed(&persona.user_id)?
        .into_iter()
        .map(|exp| {
            if let Some(overridden) = existing
                .iter()
                .find(|w| w.experience_id == exp.id && w.user_overridden)
            {
                return overridden.clone();
            }
            RoleExperienceWeight {
                id: existing
                    .iter()
                    .find(|w| w.experience_id == exp.id)
                    .map(|w| w.id.clone())
                    .unwrap_or_default(),
                persona_id: persona.id.clone(),
                experience_id: exp.id.clone(),
                relevance_score: fit_score::calculate(&persona, &exp),
                reframed_summary: None,
                highlighted_skills: vec![],
                user_overridden: false,
            }
        })
        .collect::<Vec<_>>();
    personas.save_weights(&rows)?;
    Ok(rows)
}

pub fn override_score<P: PersonaRepository>(
    personas: &P,
    persona_id: &str,
    experience_id: &str,
    score: f64,
) -> Result<RoleExperienceWeight, ApplicationError> {
    if !score.is_finite() {
        return Err(ApplicationError::Validation("score must be finite".into()));
    }
    personas.override_weight(persona_id, experience_id, fit_score::apply_override(score))
}

pub fn reset_override<E: ExperienceRepository, P: PersonaRepository>(
    experiences: &E,
    personas: &P,
    persona_id: &str,
    experience_id: &str,
) -> Result<RoleExperienceWeight, ApplicationError> {
    personas.reset_weight(persona_id, experience_id)?;
    recalculate(experiences, personas, persona_id)?
        .into_iter()
        .find(|w| w.experience_id == experience_id)
        .ok_or_else(|| ApplicationError::NotFound("experience".into()))
}
