//! Experience use cases: CC-FR-001/002/003 and first-run CC-FR-019.
use super::ports::{ApplicationError, ExperienceRepository};
use crate::domain::entities::{
    Experience, ExperienceEnrichment, ExperiencePatch, ExperienceStatus,
};

pub fn create<R: ExperienceRepository>(
    repo: &R,
    experience: &Experience,
) -> Result<(), ApplicationError> {
    validate(experience)?;
    repo.create(experience)
}
pub fn import_batch<R: ExperienceRepository>(
    repo: &R,
    experiences: &[Experience],
) -> Result<usize, ApplicationError> {
    // Validate the entire input before the first write so malformed imports never partially apply.
    for experience in experiences {
        validate(experience)?;
    }
    for experience in experiences {
        repo.create(experience)?;
    }
    Ok(experiences.len())
}
pub fn needs_onboarding<R: ExperienceRepository>(
    repo: &R,
    user_id: &str,
) -> Result<bool, ApplicationError> {
    Ok(repo.list(user_id)?.is_empty())
}
pub fn update<R: ExperienceRepository>(
    repo: &R,
    id: &str,
    expected_version: u32,
    patch: &ExperiencePatch,
) -> Result<Experience, ApplicationError> {
    if matches!(patch.title.as_deref(), Some("")) {
        return Err(ApplicationError::Validation("title is required".into()));
    }
    if let Some(next) = &patch.status {
        let current = repo
            .get(id)?
            .ok_or_else(|| ApplicationError::NotFound("experience".into()))?;
        validate_status_transition(&current.status, next)?;
    }
    repo.update(id, expected_version, patch)
}

/// Explicit lifecycle used by the existing `save_experience` command.
/// Drafts may be confirmed or discarded; confirmed records may only be archived.
pub fn validate_status_transition(
    current: &ExperienceStatus,
    next: &ExperienceStatus,
) -> Result<(), ApplicationError> {
    let valid = current == next
        || matches!(
            (current, next),
            (
                ExperienceStatus::Draft,
                ExperienceStatus::Confirmed | ExperienceStatus::Discarded
            ) | (ExperienceStatus::Confirmed, ExperienceStatus::Archived)
        );
    valid.then_some(()).ok_or_else(|| {
        ApplicationError::Conflict(format!(
            "invalid experience status transition: {current:?} -> {next:?}"
        ))
    })
}

/// Returns active experiences whose inclusive date ranges overlap `target`.
/// Missing starts/ends are treated as open ranges so the UI can warn before confirmation.
pub fn overlapping_ids(target: &Experience, values: &[Experience]) -> Vec<String> {
    let Some((start, end)) = normalized_range(target) else {
        return Vec::new();
    };
    values
        .iter()
        .filter(|other| other.id != target.id && other.user_id == target.user_id)
        .filter(|other| {
            !matches!(
                other.status,
                ExperienceStatus::Discarded | ExperienceStatus::Archived
            )
        })
        .filter(|other| {
            normalized_range(other)
                .is_some_and(|(other_start, other_end)| start <= other_end && other_start <= end)
        })
        .map(|other| other.id.clone())
        .collect()
}

// ISO calendar dates only. A missing start cannot be placed on a timeline; a
// missing end means ongoing. Reversed and invalid ranges do not produce warnings.
fn normalized_range(value: &Experience) -> Option<(i32, i32)> {
    let start = iso_date_key(value.start_date.as_deref()?)?;
    let end = match value.end_date.as_deref() {
        Some(date) => iso_date_key(date)?,
        None => i32::MAX,
    };
    (end >= start).then_some((start, end))
}
fn iso_date_key(value: &str) -> Option<i32> {
    let parts = value.split('-').collect::<Vec<_>>();
    if !(1..=3).contains(&parts.len()) || parts.iter().any(|v| v.is_empty()) {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    if !(1..=9999).contains(&year) {
        return None;
    }
    let month = match parts.get(1) {
        Some(v) => v.parse::<u32>().ok()?,
        None => 1,
    };
    if !(1..=12).contains(&month) {
        return None;
    }
    let day = match parts.get(2) {
        Some(v) => v.parse::<u32>().ok()?,
        None => 1,
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=max_day).contains(&day) {
        return None;
    }
    Some(year * 372 + (month as i32 - 1) * 31 + day as i32 - 1)
}
/// AI may update derived fields only; repository SQL cannot touch raw_description.
pub fn apply_ai_enrichment<R: ExperienceRepository>(
    repo: &R,
    id: &str,
    expected_version: u32,
    value: &ExperienceEnrichment,
) -> Result<Experience, ApplicationError> {
    repo.update_enrichment(id, expected_version, value)
}
pub fn discard<R: ExperienceRepository>(
    repo: &R,
    id: &str,
    expected_version: u32,
) -> Result<Experience, ApplicationError> {
    repo.update(
        id,
        expected_version,
        &ExperiencePatch {
            status: Some(ExperienceStatus::Discarded),
            ..Default::default()
        },
    )
}
pub fn validate(value: &Experience) -> Result<(), ApplicationError> {
    if value.id.trim().is_empty() || value.title.trim().is_empty() {
        return Err(ApplicationError::Validation(
            "id and title are required".into(),
        ));
    }
    if let (Some(start), Some(end)) = (&value.start_date, &value.end_date) {
        if end < start {
            return Err(ApplicationError::Validation(
                "end_date precedes start_date".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MemoryRepo(RefCell<Vec<Experience>>);
    impl ExperienceRepository for MemoryRepo {
        fn list(&self, user: &str) -> Result<Vec<Experience>, ApplicationError> {
            Ok(self
                .0
                .borrow()
                .iter()
                .filter(|v| v.user_id == user)
                .cloned()
                .collect())
        }
        fn list_confirmed(&self, user: &str) -> Result<Vec<Experience>, ApplicationError> {
            Ok(self
                .list(user)?
                .into_iter()
                .filter(|v| v.status == ExperienceStatus::Confirmed)
                .collect())
        }
        fn get(&self, id: &str) -> Result<Option<Experience>, ApplicationError> {
            Ok(self.0.borrow().iter().find(|v| v.id == id).cloned())
        }
        fn create(&self, value: &Experience) -> Result<(), ApplicationError> {
            if self.get(&value.id)?.is_some() {
                return Err(ApplicationError::Conflict("duplicate".into()));
            }
            self.0.borrow_mut().push(value.clone());
            Ok(())
        }
        fn update(
            &self,
            id: &str,
            version: u32,
            patch: &ExperiencePatch,
        ) -> Result<Experience, ApplicationError> {
            let mut values = self.0.borrow_mut();
            let value = values
                .iter_mut()
                .find(|v| v.id == id)
                .ok_or_else(|| ApplicationError::NotFound(id.into()))?;
            if value.version != version {
                return Err(ApplicationError::Conflict("version".into()));
            }
            if let Some(title) = &patch.title {
                value.title = title.clone()
            }
            if let Some(status) = &patch.status {
                value.status = status.clone()
            }
            value.version += 1;
            Ok(value.clone())
        }
        fn update_enrichment(
            &self,
            id: &str,
            version: u32,
            e: &ExperienceEnrichment,
        ) -> Result<Experience, ApplicationError> {
            let mut values = self.0.borrow_mut();
            let value = values
                .iter_mut()
                .find(|v| v.id == id)
                .ok_or_else(|| ApplicationError::NotFound(id.into()))?;
            if value.version != version {
                return Err(ApplicationError::Conflict("version".into()));
            }
            value.structured_achievements = e.structured_achievements.clone();
            value.skills_demonstrated = e.skills_demonstrated.clone();
            value.version += 1;
            Ok(value.clone())
        }
        fn delete(&self, id: &str, _: u32) -> Result<(), ApplicationError> {
            self.0.borrow_mut().retain(|v| v.id != id);
            Ok(())
        }
    }
    fn experience(id: &str) -> Experience {
        Experience {
            id: id.into(),
            user_id: "u".into(),
            kind: crate::domain::entities::ExperienceType::Work,
            title: "Engineer".into(),
            organization: None,
            start_date: Some("2024-01".into()),
            end_date: Some("2025-01".into()),
            raw_description: "raw".into(),
            structured_achievements: vec![],
            skills_demonstrated: vec![],
            industry_tags: vec![],
            education_level: None,
            status: ExperienceStatus::Draft,
            version: 1,
        }
    }

    #[test]
    fn crud_validation_conflict_and_not_found() {
        let repo = MemoryRepo::default();
        assert!(needs_onboarding(&repo, "u").unwrap());
        create(&repo, &experience("e")).unwrap();
        assert!(!needs_onboarding(&repo, "u").unwrap());
        assert!(matches!(
            create(&repo, &experience("e")),
            Err(ApplicationError::Conflict(_))
        ));
        let updated = update(
            &repo,
            "e",
            1,
            &ExperiencePatch {
                title: Some("Lead".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!((updated.title.as_str(), updated.version), ("Lead", 2));
        assert!(matches!(
            update(&repo, "e", 1, &ExperiencePatch::default()),
            Err(ApplicationError::Conflict(_))
        ));
        assert!(matches!(
            update(&repo, "missing", 1, &ExperiencePatch::default()),
            Err(ApplicationError::NotFound(_))
        ));
        assert!(matches!(
            update(
                &repo,
                "e",
                2,
                &ExperiencePatch {
                    title: Some("".into()),
                    ..Default::default()
                }
            ),
            Err(ApplicationError::Validation(_))
        ));
        let enriched = apply_ai_enrichment(
            &repo,
            "e",
            2,
            &ExperienceEnrichment {
                structured_achievements: vec!["A".into()],
                skills_demonstrated: vec!["Rust".into()],
            },
        )
        .unwrap();
        assert_eq!(enriched.raw_description, "raw");
        assert_eq!(
            discard(&repo, "e", 3).unwrap().status,
            ExperienceStatus::Discarded
        )
    }
    #[test]
    fn import_is_all_or_nothing_on_validation() {
        let repo = MemoryRepo::default();
        let mut invalid = experience("bad");
        invalid.title = " ".into();
        assert!(matches!(
            import_batch(&repo, &[experience("ok"), invalid]),
            Err(ApplicationError::Validation(_))
        ));
        assert!(repo.0.borrow().is_empty());
        assert_eq!(
            import_batch(&repo, &[experience("a"), experience("b")]).unwrap(),
            2
        )
    }
    #[test]
    fn rejects_bad_identity_and_date_range() {
        let mut value = experience("");
        assert!(matches!(
            validate(&value),
            Err(ApplicationError::Validation(_))
        ));
        value.id = "e".into();
        value.start_date = Some("2025-02".into());
        value.end_date = Some("2024-01".into());
        assert!(matches!(
            validate(&value),
            Err(ApplicationError::Validation(_))
        ))
    }
    #[test]
    fn lifecycle_and_overlap_warnings_are_explicit() {
        let repo = MemoryRepo::default();
        create(&repo, &experience("draft")).unwrap();
        assert_eq!(
            update(
                &repo,
                "draft",
                1,
                &ExperiencePatch {
                    status: Some(ExperienceStatus::Confirmed),
                    ..Default::default()
                }
            )
            .unwrap()
            .status,
            ExperienceStatus::Confirmed
        );
        assert!(matches!(
            update(
                &repo,
                "draft",
                2,
                &ExperiencePatch {
                    status: Some(ExperienceStatus::Draft),
                    ..Default::default()
                }
            ),
            Err(ApplicationError::Conflict(_))
        ));
        let mut current = experience("current");
        current.start_date = Some("2024-06".into());
        current.end_date = None;
        let mut prior = experience("prior");
        prior.start_date = Some("2024-01".into());
        prior.end_date = Some("2024-12".into());
        let mut old = experience("old");
        old.start_date = Some("2020-01".into());
        old.end_date = Some("2021-01".into());
        assert_eq!(overlapping_ids(&current, &[prior, old]), vec!["prior"]);
    }
    #[test]
    fn overlap_ignores_missing_invalid_and_reversed_ranges() {
        let mut target = experience("target");
        target.start_date = Some("2024-02-29".into());
        target.end_date = None;
        let mut missing = experience("missing");
        missing.start_date = None;
        missing.end_date = Some("2025-01".into());
        let mut invalid = experience("invalid");
        invalid.start_date = Some("2024-13-01".into());
        invalid.end_date = None;
        let mut reversed = experience("reversed");
        reversed.start_date = Some("2025-01".into());
        reversed.end_date = Some("2024-01".into());
        let mut valid = experience("valid");
        valid.start_date = Some("2024-03".into());
        valid.end_date = Some("2024-04".into());
        assert_eq!(
            overlapping_ids(&target, &[missing, invalid, reversed, valid]),
            vec!["valid"]
        );
        target.start_date = Some("2023-02-29".into());
        assert!(overlapping_ids(&target, &[]).is_empty());
    }
}
