//! Resume aggregation/versioning and confirmed conversational tuning use cases.
//! Requirement mapping: CC-FR-006, CC-FR-007, CC-FR-018.

use crate::domain::entities::{Experience, ExperienceType, RoleExperienceWeight};
use crate::domain::resume::{
    diff, ResumeDiff, ResumeEdit, ResumeError, ResumeRenderData, ResumeTemplate, ResumeVersion,
};

pub const MAX_VERSIONS_PER_PERSONA: usize = 5;

/// Selects persona-relevant work deterministically. Manual overrides are already
/// represented by the effective repository score passed in `weights`.
pub fn select_work_experiences(
    values: &[Experience],
    weights: &[RoleExperienceWeight],
    max_experiences: usize,
) -> Vec<Experience> {
    let score = |id: &str| {
        weights
            .iter()
            .find(|w| w.experience_id == id)
            .map(|w| w.relevance_score)
            .unwrap_or(0.0)
    };
    let has_positive = values
        .iter()
        .filter(|e| matches!(e.kind, ExperienceType::Work | ExperienceType::Project))
        .any(|e| score(&e.id) > 0.0);
    let mut selected = values
        .iter()
        .filter(|e| matches!(e.kind, ExperienceType::Work | ExperienceType::Project))
        .filter(|e| !has_positive || score(&e.id) > 0.0)
        .cloned()
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| {
        score(&b.id)
            .total_cmp(&score(&a.id))
            .then_with(|| b.start_date.cmp(&a.start_date))
            .then_with(|| a.id.cmp(&b.id))
    });
    selected.truncate(max_experiences);
    selected
}

/// Higher weight → more achievement bullets kept on the rendered resume.
pub fn allocate_achievements_by_weight(achievements: &[String], score: f64) -> Vec<String> {
    if achievements.is_empty() || !score.is_finite() || score <= 0.0 {
        return Vec::new();
    }
    let n = achievements.len();
    let target = if score >= 0.75 {
        n
    } else if score >= 0.5 {
        // Keep about two-thirds, but never ask for more than available.
        ((n * 2 + 2) / 3).max(1)
    } else if score >= 0.25 {
        n.div_ceil(2).max(1)
    } else {
        1
    };
    achievements.iter().take(target.min(n)).cloned().collect()
}

pub trait ResumeRepository {
    fn list(&self, persona_id: &str) -> Result<Vec<ResumeVersion>, ResumeError>;
    fn get(&self, id: &str) -> Result<Option<ResumeVersion>, ResumeError>;
    fn save(&self, version: &ResumeVersion) -> Result<(), ResumeError>;
    fn delete(&self, id: &str) -> Result<(), ResumeError>;
}

pub struct ResumeService<R> {
    repository: R,
}

impl<R: ResumeRepository> ResumeService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn create_version(&self, mut version: ResumeVersion) -> Result<ResumeVersion, ResumeError> {
        let existing = self.repository.list(&version.persona_id)?;
        if existing.len() >= MAX_VERSIONS_PER_PERSONA {
            return Err(ResumeError::VersionLimit);
        }
        version.data.normalize();
        version.data.validate()?;
        version.revision = existing.iter().map(|item| item.revision).max().unwrap_or(0) + 1;
        self.repository.save(&version)?;
        Ok(version)
    }

    pub fn compare(&self, left_id: &str, right_id: &str) -> Result<ResumeDiff, ResumeError> {
        let left = self.repository.get(left_id)?.ok_or(ResumeError::NotFound)?;
        let right = self
            .repository
            .get(right_id)?
            .ok_or(ResumeError::NotFound)?;
        Ok(diff(&left, &right))
    }

    /// Applies only an explicitly confirmed edit. Suggested AI text is held by the
    /// caller until confirmation, so the original version remains immutable.
    pub fn confirm_tune(
        &self,
        base_id: &str,
        new_id: String,
        label: String,
        edit: ResumeEdit,
        created_at: String,
    ) -> Result<ResumeVersion, ResumeError> {
        let base = self.repository.get(base_id)?.ok_or(ResumeError::NotFound)?;
        let mut data = base.data.clone();
        edit.apply(&mut data)?;
        self.create_version(ResumeVersion {
            id: new_id,
            persona_id: base.persona_id,
            label,
            template: base.template,
            revision: 0,
            data,
            parent_id: Some(base.id),
            created_at,
        })
    }

    /// Restoring creates another immutable version instead of overwriting history.
    pub fn restore(
        &self,
        source_id: &str,
        new_id: String,
        created_at: String,
    ) -> Result<ResumeVersion, ResumeError> {
        let source = self
            .repository
            .get(source_id)?
            .ok_or(ResumeError::NotFound)?;
        self.create_version(ResumeVersion {
            id: new_id,
            label: format!("{} (restored)", source.label),
            revision: 0,
            parent_id: Some(source.id.clone()),
            created_at,
            ..source
        })
    }
}

pub fn aggregate(
    header: crate::domain::resume::ResumeHeader,
    summary: Option<String>,
    experience: Vec<crate::domain::resume::ResumeEntry>,
    education: Vec<crate::domain::resume::ResumeEntry>,
    skills: Vec<String>,
) -> ResumeRenderData {
    let mut data = ResumeRenderData {
        header,
        summary,
        experience,
        education,
        skills,
        ..Default::default()
    };
    data.normalize();
    data
}

pub fn choose_template(id: &str) -> Result<ResumeTemplate, ResumeError> {
    ResumeTemplate::ALL
        .into_iter()
        .find(|template| template.id() == id)
        .ok_or_else(|| ResumeError::Validation(format!("unknown resume template: {id}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resume::{ResumeHeader, ResumeRenderData};
    use std::cell::RefCell;

    #[derive(Default)]
    struct MemoryRepository {
        versions: RefCell<Vec<ResumeVersion>>,
    }
    impl ResumeRepository for MemoryRepository {
        fn list(&self, persona_id: &str) -> Result<Vec<ResumeVersion>, ResumeError> {
            Ok(self
                .versions
                .borrow()
                .iter()
                .filter(|item| item.persona_id == persona_id)
                .cloned()
                .collect())
        }
        fn get(&self, id: &str) -> Result<Option<ResumeVersion>, ResumeError> {
            Ok(self
                .versions
                .borrow()
                .iter()
                .find(|item| item.id == id)
                .cloned())
        }
        fn save(&self, version: &ResumeVersion) -> Result<(), ResumeError> {
            self.versions.borrow_mut().push(version.clone());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<(), ResumeError> {
            self.versions.borrow_mut().retain(|item| item.id != id);
            Ok(())
        }
    }

    fn version(id: &str) -> ResumeVersion {
        ResumeVersion {
            id: id.into(),
            persona_id: "p1".into(),
            label: id.into(),
            template: ResumeTemplate::Classic,
            revision: 0,
            parent_id: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            data: ResumeRenderData {
                header: ResumeHeader {
                    full_name: "Ada".into(),
                    headline: "Engineer".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn confirmed_tune_creates_child_and_preserves_base() {
        let service = ResumeService::new(MemoryRepository::default());
        service.create_version(version("base")).unwrap();
        let child = service
            .confirm_tune(
                "base",
                "tuned".into(),
                "Tuned".into(),
                ResumeEdit::SetHeadline {
                    before: "Engineer".into(),
                    after: "Staff Engineer".into(),
                },
                "2026-01-02T00:00:00Z".into(),
            )
            .unwrap();
        assert_eq!(child.parent_id.as_deref(), Some("base"));
        assert_eq!(child.data.header.headline, "Staff Engineer");
        assert_eq!(
            service
                .repository
                .get("base")
                .unwrap()
                .unwrap()
                .data
                .header
                .headline,
            "Engineer"
        );
    }

    #[test]
    fn rejects_sixth_version_for_same_persona() {
        let service = ResumeService::new(MemoryRepository::default());
        for index in 0..MAX_VERSIONS_PER_PERSONA {
            service
                .create_version(version(&format!("v{index}")))
                .unwrap();
        }
        assert_eq!(
            service.create_version(version("overflow")),
            Err(ResumeError::VersionLimit)
        );
    }

    #[test]
    fn compare_restore_and_missing_paths() {
        let service = ResumeService::new(MemoryRepository::default());
        service.create_version(version("left")).unwrap();
        let mut right = version("right");
        right.data.header.headline = "Principal Engineer".into();
        service.create_version(right).unwrap();
        assert!(service.compare("left", "right").unwrap().header_changed);
        assert_eq!(
            service.compare("missing", "right"),
            Err(ResumeError::NotFound)
        );
        assert_eq!(
            service.confirm_tune(
                "missing",
                "x".into(),
                "X".into(),
                ResumeEdit::SetHeadline {
                    before: "a".into(),
                    after: "b".into()
                },
                "now".into()
            ),
            Err(ResumeError::NotFound)
        );
        let restored = service
            .restore("left", "restored".into(), "later".into())
            .unwrap();
        assert_eq!(restored.parent_id.as_deref(), Some("left"));
        assert_eq!(
            service.restore("missing", "x".into(), "later".into()),
            Err(ResumeError::NotFound)
        );
    }

    #[test]
    fn aggregate_normalizes_and_template_selection_validates() {
        let data = aggregate(
            ResumeHeader {
                full_name: " Ada ".into(),
                headline: " Engineer ".into(),
                ..Default::default()
            },
            Some(" Summary ".into()),
            vec![],
            vec![],
            vec![" Rust ".into(), "Rust".into()],
        );
        assert_eq!(data.skills, vec!["Rust"]);
        assert_eq!(choose_template("classic").unwrap(), ResumeTemplate::Classic);
        assert!(matches!(
            choose_template("unknown"),
            Err(ResumeError::Validation(_))
        ));
    }
    #[test]
    fn fit_selection_honors_override_score_limit_and_stable_ties() {
        let a = crate::domain::entities::Experience {
            id: "a".into(),
            user_id: "u".into(),
            kind: ExperienceType::Work,
            title: "A".into(),
            organization: None,
            start_date: Some("2023-01".into()),
            end_date: None,
            raw_description: String::new(),
            structured_achievements: vec![],
            skills_demonstrated: vec![],
            industry_tags: vec![],
            education_level: None,
            status: crate::domain::entities::ExperienceStatus::Confirmed,
            version: 1,
        };
        let mut b = a.clone();
        b.id = "b".into();
        b.start_date = Some("2024-01".into());
        let mut c = a.clone();
        c.id = "c".into();
        let weights = vec![
            RoleExperienceWeight {
                id: "wa".into(),
                persona_id: "p".into(),
                experience_id: "a".into(),
                relevance_score: 0.9,
                reframed_summary: None,
                highlighted_skills: vec![],
                user_overridden: true,
            },
            RoleExperienceWeight {
                id: "wb".into(),
                persona_id: "p".into(),
                experience_id: "b".into(),
                relevance_score: 0.8,
                reframed_summary: None,
                highlighted_skills: vec![],
                user_overridden: false,
            },
            RoleExperienceWeight {
                id: "wc".into(),
                persona_id: "p".into(),
                experience_id: "c".into(),
                relevance_score: 0.0,
                reframed_summary: None,
                highlighted_skills: vec![],
                user_overridden: false,
            },
        ];
        assert_eq!(
            select_work_experiences(&[a, b, c], &weights, 2)
                .into_iter()
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        )
    }
    #[test]
    fn education_positive_score_does_not_filter_zero_score_work() {
        let work = crate::domain::entities::Experience {
            id: "work".into(),
            user_id: "u".into(),
            kind: ExperienceType::Work,
            title: "Work".into(),
            organization: None,
            start_date: Some("2024-01".into()),
            end_date: None,
            raw_description: String::new(),
            structured_achievements: vec![],
            skills_demonstrated: vec![],
            industry_tags: vec![],
            education_level: None,
            status: crate::domain::entities::ExperienceStatus::Confirmed,
            version: 1,
        };
        let mut education = work.clone();
        education.id = "education".into();
        education.kind = ExperienceType::Education;
        let weights = vec![RoleExperienceWeight {
            id: "we".into(),
            persona_id: "p".into(),
            experience_id: "education".into(),
            relevance_score: 1.0,
            reframed_summary: None,
            highlighted_skills: vec![],
            user_overridden: false,
        }];
        assert_eq!(
            select_work_experiences(&[work, education], &weights, 5)
                .into_iter()
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            vec!["work"]
        )
    }

    #[test]
    fn allocate_achievements_scales_with_weight() {
        let items = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
        ];
        assert_eq!(allocate_achievements_by_weight(&items, 0.9).len(), 4);
        assert_eq!(allocate_achievements_by_weight(&items, 0.6).len(), 3);
        assert_eq!(allocate_achievements_by_weight(&items, 0.3).len(), 2);
        assert_eq!(allocate_achievements_by_weight(&items, 0.1), vec!["a".to_string()]);
        assert!(allocate_achievements_by_weight(&items, 0.0).is_empty());
        // Single bullet must never panic (previously clamp(2,1) crashed generate).
        assert_eq!(
            allocate_achievements_by_weight(&["only".into()], 0.6),
            vec!["only".to_string()]
        );
    }
}
