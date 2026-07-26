pub mod application {
    pub mod experience_structuring;
    pub mod experiences;
    pub mod fit_scores;
    pub mod jobs;
    pub mod llm_orchestration;
    pub mod personas;
    pub mod ports;
    pub mod resumes;
    pub mod skills;
}
#[cfg(feature = "desktop")]
pub mod bootstrap;
pub mod domain {
    pub mod entities;
    pub mod experience_structuring;
    pub mod fit_score;
    pub mod jobs;
    pub mod llm;
    pub mod resume;
    pub mod skills;
    pub mod skills_learning;
}
pub mod error;
pub mod infra;
pub mod interface;
#[cfg(feature = "desktop")]
pub use bootstrap::run;
