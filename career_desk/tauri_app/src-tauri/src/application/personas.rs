//! Persona use cases: CC-FR-004. Deletion removes role links by FK, never experiences.
use super::ports::{ApplicationError, PersonaRepository};
use crate::domain::entities::{Persona, PersonaPatch};
pub fn create<R: PersonaRepository>(repo: &R, persona: &Persona) -> Result<(), ApplicationError> {
    validate(persona)?;
    repo.create(persona)
}
pub fn update<R: PersonaRepository>(
    repo: &R,
    id: &str,
    patch: &PersonaPatch,
) -> Result<Persona, ApplicationError> {
    if matches!(patch.name.as_deref(), Some("")) {
        return Err(ApplicationError::Validation("name is required".into()));
    }
    if matches!(patch.max_experiences, Some(0)) {
        return Err(ApplicationError::Validation(
            "max_experiences must be positive".into(),
        ));
    }
    repo.update(id, patch)
}
fn validate(value: &Persona) -> Result<(), ApplicationError> {
    if value.id.trim().is_empty() || value.name.trim().is_empty() || value.max_experiences == 0 {
        return Err(ApplicationError::Validation("invalid persona".into()));
    }
    if value
        .capability_weights
        .iter()
        .any(|(_, w)| !w.is_finite() || *w < 0.0 || *w > 1.0)
    {
        return Err(ApplicationError::Validation(
            "capability weight outside 0..1".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::RoleExperienceWeight;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Repo(RefCell<Vec<Persona>>);
    impl PersonaRepository for Repo {
        fn list(&self, u: &str) -> Result<Vec<Persona>, ApplicationError> {
            Ok(self
                .0
                .borrow()
                .iter()
                .filter(|v| v.user_id == u)
                .cloned()
                .collect())
        }
        fn get(&self, id: &str) -> Result<Option<Persona>, ApplicationError> {
            Ok(self.0.borrow().iter().find(|v| v.id == id).cloned())
        }
        fn create(&self, v: &Persona) -> Result<(), ApplicationError> {
            if self.get(&v.id)?.is_some() {
                return Err(ApplicationError::Conflict("duplicate".into()));
            }
            self.0.borrow_mut().push(v.clone());
            Ok(())
        }
        fn update(&self, id: &str, p: &PersonaPatch) -> Result<Persona, ApplicationError> {
            let mut values = self.0.borrow_mut();
            let v = values
                .iter_mut()
                .find(|v| v.id == id)
                .ok_or_else(|| ApplicationError::NotFound(id.into()))?;
            if let Some(name) = &p.name {
                v.name = name.clone()
            }
            if let Some(max) = p.max_experiences {
                v.max_experiences = max
            }
            Ok(v.clone())
        }
        fn delete(&self, id: &str) -> Result<(), ApplicationError> {
            self.0.borrow_mut().retain(|v| v.id != id);
            Ok(())
        }
        fn get_weights(&self, _: &str) -> Result<Vec<RoleExperienceWeight>, ApplicationError> {
            Ok(vec![])
        }
        fn save_weights(&self, _: &[RoleExperienceWeight]) -> Result<(), ApplicationError> {
            Ok(())
        }
        fn override_weight(
            &self,
            _: &str,
            _: &str,
            _: f64,
        ) -> Result<RoleExperienceWeight, ApplicationError> {
            Err(ApplicationError::NotFound("weight".into()))
        }
        fn reset_weight(&self, _: &str, _: &str) -> Result<(), ApplicationError> {
            Ok(())
        }
    }
    fn persona(id: &str) -> Persona {
        Persona {
            id: id.into(),
            user_id: "u".into(),
            name: "Role".into(),
            is_default: false,
            identity_statement: None,
            career_narrative: None,
            tone_style: None,
            capability_weights: vec![("tech".into(), 0.5)],
            target_job_profiles: vec![],
            max_experiences: 5,
            preferred_model: None,
        }
    }
    #[test]
    fn create_update_validation_conflict_not_found() {
        let repo = Repo::default();
        create(&repo, &persona("p")).unwrap();
        assert!(matches!(
            create(&repo, &persona("p")),
            Err(ApplicationError::Conflict(_))
        ));
        assert_eq!(
            update(
                &repo,
                "p",
                &PersonaPatch {
                    name: Some("Lead".into()),
                    ..Default::default()
                }
            )
            .unwrap()
            .name,
            "Lead"
        );
        assert!(matches!(
            update(&repo, "missing", &PersonaPatch::default()),
            Err(ApplicationError::NotFound(_))
        ));
        assert!(matches!(
            update(
                &repo,
                "p",
                &PersonaPatch {
                    name: Some("".into()),
                    ..Default::default()
                }
            ),
            Err(ApplicationError::Validation(_))
        ));
        assert!(matches!(
            update(
                &repo,
                "p",
                &PersonaPatch {
                    max_experiences: Some(0),
                    ..Default::default()
                }
            ),
            Err(ApplicationError::Validation(_))
        ))
    }
    #[test]
    fn rejects_invalid_persona_and_weights() {
        for mut value in [persona(""), persona("x")] {
            if value.id == "x" {
                value.capability_weights = vec![("bad".into(), 1.1)]
            }
            assert!(matches!(
                create(&Repo::default(), &value),
                Err(ApplicationError::Validation(_))
            ))
        }
        let mut nan = persona("n");
        nan.capability_weights = vec![("bad".into(), f64::NAN)];
        assert!(matches!(
            create(&Repo::default(), &nan),
            Err(ApplicationError::Validation(_))
        ))
    }
}
