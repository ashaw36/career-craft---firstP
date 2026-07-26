//! Learning path/progress model and completed-item conversion (CC-FR-015/016).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathSource {
    SkillGraph,
    JobGap,
    Manual,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathStatus {
    Active,
    Completed,
    Archived,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearningItem {
    pub id: String,
    pub skill_id: String,
    pub title: String,
    pub resource_url: Option<String>,
    pub estimated_hours: u16,
    pub status: ItemStatus,
    pub completion_note: Option<String>,
    pub converted_experience_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearningPath {
    pub id: String,
    pub persona_id: String,
    pub target_gap: String,
    pub items: Vec<LearningItem>,
    pub source: PathSource,
    pub status: PathStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LearningError {
    NotFound,
    InvalidTransition,
    EmptyPath,
    AlreadyConverted,
    CompletionNoteRequired,
}

impl LearningPath {
    pub fn progress_percent(&self) -> u8 {
        if self.items.is_empty() {
            return 0;
        }
        let done = self
            .items
            .iter()
            .filter(|v| matches!(v.status, ItemStatus::Completed | ItemStatus::Skipped))
            .count();
        ((done * 100) as f32 / self.items.len() as f32).round() as u8
    }
    pub fn update_item(
        &mut self,
        item_id: &str,
        next: ItemStatus,
        note: Option<String>,
    ) -> Result<(), LearningError> {
        let item = self
            .items
            .iter_mut()
            .find(|v| v.id == item_id)
            .ok_or(LearningError::NotFound)?;
        let valid = item.status == next
            || matches!(
                (&item.status, &next),
                (
                    ItemStatus::Pending,
                    ItemStatus::InProgress | ItemStatus::Completed | ItemStatus::Skipped
                ) | (
                    ItemStatus::InProgress,
                    ItemStatus::Completed | ItemStatus::Skipped
                )
            );
        if !valid {
            return Err(LearningError::InvalidTransition);
        }
        if next == ItemStatus::Completed && note.as_deref().is_none_or(|v| v.trim().is_empty()) {
            return Err(LearningError::CompletionNoteRequired);
        }
        item.status = next;
        if note.is_some() {
            item.completion_note = note;
        }
        if self
            .items
            .iter()
            .all(|v| matches!(v.status, ItemStatus::Completed | ItemStatus::Skipped))
        {
            self.status = PathStatus::Completed;
        }
        Ok(())
    }
    pub fn mark_converted(
        &mut self,
        item_id: &str,
        experience_id: String,
    ) -> Result<(), LearningError> {
        let item = self
            .items
            .iter_mut()
            .find(|v| v.id == item_id)
            .ok_or(LearningError::NotFound)?;
        if item.status != ItemStatus::Completed {
            return Err(LearningError::InvalidTransition);
        }
        if item.converted_experience_id.is_some() {
            return Err(LearningError::AlreadyConverted);
        }
        item.converted_experience_id = Some(experience_id);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionExperienceDraft {
    pub title: String,
    pub raw_description: String,
    pub skills_demonstrated: Vec<String>,
    pub source_learning_item_id: String,
}

pub fn completion_to_experience(
    item: &LearningItem,
) -> Result<CompletionExperienceDraft, LearningError> {
    if item.status != ItemStatus::Completed {
        return Err(LearningError::InvalidTransition);
    }
    if item.converted_experience_id.is_some() {
        return Err(LearningError::AlreadyConverted);
    }
    let note = item
        .completion_note
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or(LearningError::CompletionNoteRequired)?;
    Ok(CompletionExperienceDraft {
        title: format!("学习成果：{}", item.title),
        raw_description: note.to_owned(),
        skills_demonstrated: vec![item.skill_id.clone()],
        source_learning_item_id: item.id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn path() -> LearningPath {
        LearningPath {
            id: "p".into(),
            persona_id: "u".into(),
            target_gap: "Rust".into(),
            source: PathSource::Manual,
            status: PathStatus::Active,
            items: vec![LearningItem {
                id: "i".into(),
                skill_id: "rust".into(),
                title: "Build".into(),
                resource_url: None,
                estimated_hours: 2,
                status: ItemStatus::Pending,
                completion_note: None,
                converted_experience_id: None,
            }],
        }
    }
    #[test]
    fn completion_updates_progress_and_path() {
        let mut p = path();
        p.update_item("i", ItemStatus::Completed, Some("Built a CLI".into()))
            .unwrap();
        assert_eq!(p.progress_percent(), 100);
        assert_eq!(p.status, PathStatus::Completed);
    }
    #[test]
    fn conversion_is_explicit_and_once_only() {
        let mut p = path();
        p.update_item("i", ItemStatus::Completed, Some("Built a CLI".into()))
            .unwrap();
        let draft = completion_to_experience(&p.items[0]).unwrap();
        assert_eq!(draft.raw_description, "Built a CLI");
        p.mark_converted("i", "exp".into()).unwrap();
        assert_eq!(
            completion_to_experience(&p.items[0]),
            Err(LearningError::AlreadyConverted)
        );
    }
}
