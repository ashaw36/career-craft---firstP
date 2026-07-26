//! Skill graph and learning use-case ports (CC-FR-013..017).
use crate::domain::skills::{Skill, SkillError, SkillOrigin};
use crate::domain::skills_learning::LearningPath;

pub trait SkillRepository {
    fn list(&self, owner_id: &str) -> Result<Vec<Skill>, SkillError>;
    fn get(&self, id: &str) -> Result<Option<Skill>, SkillError>;
    fn create_custom(&self, skill: &Skill) -> Result<(), SkillError>;
    fn update_custom(&self, skill: &Skill) -> Result<(), SkillError>;
    fn delete_custom(&self, id: &str, owner_id: &str) -> Result<(), SkillError>;
}

pub trait LearningPathRepository {
    fn list(&self, persona_id: &str) -> Result<Vec<LearningPath>, SkillError>;
    fn get(&self, id: &str) -> Result<Option<LearningPath>, SkillError>;
    fn save(&self, path: &LearningPath) -> Result<(), SkillError>;
    fn delete(&self, id: &str) -> Result<(), SkillError>;
}

pub fn create_custom<R: SkillRepository>(repo: &R, skill: &Skill) -> Result<(), SkillError> {
    skill.validate()?;
    if !matches!(skill.origin, SkillOrigin::Custom { .. }) {
        return Err(SkillError::BuiltInImmutable);
    }
    ensure_unique_name(repo, skill, None)?;
    repo.create_custom(skill)
}

pub fn update_custom<R: SkillRepository>(repo: &R, skill: &Skill) -> Result<(), SkillError> {
    skill.validate()?;
    let owner = match &skill.origin {
        SkillOrigin::Custom { owner_id } => owner_id,
        SkillOrigin::BuiltIn => return Err(SkillError::BuiltInImmutable),
    };
    let existing = repo.get(&skill.id)?.ok_or(SkillError::NotFound)?;
    if existing.origin
        != (SkillOrigin::Custom {
            owner_id: owner.clone(),
        })
    {
        return Err(SkillError::BuiltInImmutable);
    }
    ensure_unique_name(repo, skill, Some(&skill.id))?;
    repo.update_custom(skill)
}

pub fn delete_custom<R: SkillRepository>(
    repo: &R,
    id: &str,
    owner_id: &str,
) -> Result<(), SkillError> {
    let existing = repo.get(id)?.ok_or(SkillError::NotFound)?;
    match existing.origin {
        SkillOrigin::BuiltIn => Err(SkillError::BuiltInImmutable),
        SkillOrigin::Custom { owner_id: owner } if owner != owner_id => Err(SkillError::NotFound),
        SkillOrigin::Custom { .. } => repo.delete_custom(id, owner_id),
    }
}

fn ensure_unique_name<R: SkillRepository>(
    repo: &R,
    skill: &Skill,
    except_id: Option<&str>,
) -> Result<(), SkillError> {
    let owner = match &skill.origin {
        SkillOrigin::Custom { owner_id } => owner_id,
        SkillOrigin::BuiltIn => return Err(SkillError::BuiltInImmutable),
    };
    let duplicate = repo.list(owner)?.iter().any(|v| {
        Some(v.id.as_str()) != except_id && v.name.trim().eq_ignore_ascii_case(skill.name.trim())
    });
    if duplicate {
        Err(SkillError::DuplicateName)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Repo(RefCell<Vec<Skill>>);
    impl SkillRepository for Repo {
        fn list(&self, owner: &str) -> Result<Vec<Skill>, SkillError> {
            Ok(self
                .0
                .borrow()
                .iter()
                .filter(|v| match &v.origin {
                    SkillOrigin::BuiltIn => true,
                    SkillOrigin::Custom { owner_id } => owner_id == owner,
                })
                .cloned()
                .collect())
        }
        fn get(&self, id: &str) -> Result<Option<Skill>, SkillError> {
            Ok(self.0.borrow().iter().find(|v| v.id == id).cloned())
        }
        fn create_custom(&self, v: &Skill) -> Result<(), SkillError> {
            self.0.borrow_mut().push(v.clone());
            Ok(())
        }
        fn update_custom(&self, v: &Skill) -> Result<(), SkillError> {
            let mut values = self.0.borrow_mut();
            let item = values
                .iter_mut()
                .find(|x| x.id == v.id)
                .ok_or(SkillError::NotFound)?;
            *item = v.clone();
            Ok(())
        }
        fn delete_custom(&self, id: &str, _: &str) -> Result<(), SkillError> {
            self.0.borrow_mut().retain(|v| v.id != id);
            Ok(())
        }
    }
    #[derive(Default)]
    struct PathRepo(RefCell<Vec<LearningPath>>);
    impl LearningPathRepository for PathRepo {
        fn list(&self, p: &str) -> Result<Vec<LearningPath>, SkillError> {
            Ok(self
                .0
                .borrow()
                .iter()
                .filter(|v| v.persona_id == p)
                .cloned()
                .collect())
        }
        fn get(&self, id: &str) -> Result<Option<LearningPath>, SkillError> {
            Ok(self.0.borrow().iter().find(|v| v.id == id).cloned())
        }
        fn save(&self, v: &LearningPath) -> Result<(), SkillError> {
            self.0.borrow_mut().push(v.clone());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<(), SkillError> {
            self.0.borrow_mut().retain(|v| v.id != id);
            Ok(())
        }
    }
    fn custom(id: &str, name: &str, owner: &str) -> Skill {
        Skill {
            id: id.into(),
            name: name.into(),
            category: "engineering".into(),
            description: String::new(),
            aliases: vec![],
            prerequisites: vec![],
            level: 1,
            resources: vec![],
            origin: SkillOrigin::Custom {
                owner_id: owner.into(),
            },
        }
    }
    #[test]
    fn custom_skill_crud_and_duplicate_rules() {
        let repo = Repo::default();
        create_custom(&repo, &custom("a", "Rust", "u")).unwrap();
        assert_eq!(
            create_custom(&repo, &custom("b", " rust ", "u")),
            Err(SkillError::DuplicateName)
        );
        let mut updated = custom("a", "Rust Advanced", "u");
        updated.level = 2;
        update_custom(&repo, &updated).unwrap();
        assert_eq!(repo.get("a").unwrap().unwrap().level, 2);
        assert_eq!(
            update_custom(&repo, &custom("missing", "X", "u")),
            Err(SkillError::NotFound)
        );
        assert_eq!(
            delete_custom(&repo, "a", "other"),
            Err(SkillError::NotFound)
        );
        delete_custom(&repo, "a", "u").unwrap();
        assert!(repo.get("a").unwrap().is_none())
    }
    #[test]
    fn builtins_are_immutable_and_invalid_custom_is_rejected() {
        let repo = Repo::default();
        let mut built = custom("built", "Built", "u");
        built.origin = SkillOrigin::BuiltIn;
        repo.0.borrow_mut().push(built.clone());
        assert_eq!(
            create_custom(&repo, &built),
            Err(SkillError::BuiltInImmutable)
        );
        assert_eq!(
            update_custom(&repo, &built),
            Err(SkillError::BuiltInImmutable)
        );
        assert_eq!(
            delete_custom(&repo, "built", "u"),
            Err(SkillError::BuiltInImmutable)
        );
        let mut invalid = custom("bad", "", "u");
        assert!(matches!(
            create_custom(&repo, &invalid),
            Err(SkillError::Invalid(_))
        ));
        invalid.name = "Bad".into();
        invalid.level = 0;
        assert!(matches!(
            create_custom(&repo, &invalid),
            Err(SkillError::Invalid(_))
        ))
    }
    #[test]
    fn cannot_take_over_another_owners_skill() {
        let repo = Repo::default();
        create_custom(&repo, &custom("a", "Rust", "owner-a")).unwrap();
        assert_eq!(
            update_custom(&repo, &custom("a", "Rust", "owner-b")),
            Err(SkillError::BuiltInImmutable)
        )
    }
    #[test]
    fn learning_path_repository_contract() {
        let repo = PathRepo::default();
        let path = LearningPath {
            id: "lp".into(),
            persona_id: "p".into(),
            target_gap: "Rust".into(),
            items: vec![],
            source: crate::domain::skills_learning::PathSource::Manual,
            status: crate::domain::skills_learning::PathStatus::Active,
        };
        repo.save(&path).unwrap();
        assert_eq!(repo.list("p").unwrap(), vec![path.clone()]);
        assert_eq!(repo.get("lp").unwrap(), Some(path));
        repo.delete("lp").unwrap();
        assert!(repo.get("lp").unwrap().is_none())
    }
}
