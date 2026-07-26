//! Application boundaries; infra implementations are injected by bootstrap.
//! CC-NFR-002..005 and CC-SEC-001.

use crate::domain::entities::{
    Experience, ExperienceEnrichment, ExperiencePatch, Persona, PersonaPatch, RoleExperienceWeight,
};

pub trait ExperienceRepository {
    fn list(&self, user_id: &str) -> Result<Vec<Experience>, ApplicationError>;
    fn list_confirmed(&self, user_id: &str) -> Result<Vec<Experience>, ApplicationError>;
    fn get(&self, id: &str) -> Result<Option<Experience>, ApplicationError>;
    fn create(&self, experience: &Experience) -> Result<(), ApplicationError>;
    fn update(
        &self,
        id: &str,
        expected_version: u32,
        patch: &ExperiencePatch,
    ) -> Result<Experience, ApplicationError>;
    fn update_enrichment(
        &self,
        id: &str,
        expected_version: u32,
        enrichment: &ExperienceEnrichment,
    ) -> Result<Experience, ApplicationError>;
    fn delete(&self, id: &str, expected_version: u32) -> Result<(), ApplicationError>;
}
pub trait PersonaRepository {
    fn list(&self, user_id: &str) -> Result<Vec<Persona>, ApplicationError>;
    fn get(&self, id: &str) -> Result<Option<Persona>, ApplicationError>;
    fn create(&self, persona: &Persona) -> Result<(), ApplicationError>;
    fn update(&self, id: &str, patch: &PersonaPatch) -> Result<Persona, ApplicationError>;
    fn delete(&self, id: &str) -> Result<(), ApplicationError>;
    fn get_weights(&self, persona_id: &str) -> Result<Vec<RoleExperienceWeight>, ApplicationError>;
    fn save_weights(&self, weights: &[RoleExperienceWeight]) -> Result<(), ApplicationError>;
    fn override_weight(
        &self,
        persona_id: &str,
        experience_id: &str,
        score: f64,
    ) -> Result<RoleExperienceWeight, ApplicationError>;
    fn reset_weight(&self, persona_id: &str, experience_id: &str) -> Result<(), ApplicationError>;
}
pub trait SecretStore {
    fn put(&self, provider: &str, secret: &str) -> Result<(), ApplicationError>;
    fn get(&self, provider: &str) -> Result<String, ApplicationError>;
    fn exists(&self, provider: &str) -> Result<bool, ApplicationError>;
    fn delete(&self, provider: &str) -> Result<(), ApplicationError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ApplicationError {
    Validation(String),
    NotFound(String),
    Conflict(String),
    Cancelled,
    Unavailable(String),
    Internal,
}
