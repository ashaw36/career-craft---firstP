//! Persistent two-phase update recovery journal (CC-FR-024 / CC-SEC-006).
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Downloaded,
    PendingInstall,
    AwaitingHealth,
    Healthy,
    RecoveryRequired,
    RolledBack,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedArtifact {
    pub version: String,
    pub path: PathBuf,
    pub sha256: String,
    pub signature_verified: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseRecovery {
    pub database_path: PathBuf,
    pub backup_path: PathBuf,
    pub previous_max_schema_version: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateJournal {
    pub current_version: String,
    pub target_version: String,
    pub downloaded_sha256: String,
    pub previous_artifact: SignedArtifact,
    pub database: DatabaseRecovery,
    pub phase: Phase,
    pub health_attempts: u8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupDecision {
    None,
    ProceedHealthCheck,
    Recover(Box<UpdateJournal>),
}
pub trait RecoveryInstaller {
    fn restore(&self, artifact: &SignedArtifact, destination: &Path) -> Result<(), AppError>;
}
pub struct VerifiedFileInstaller;
impl RecoveryInstaller for VerifiedFileInstaller {
    fn restore(&self, artifact: &SignedArtifact, destination: &Path) -> Result<(), AppError> {
        if !artifact.signature_verified {
            return Err(AppError::Unavailable(
                "previous artifact lacks trusted signature verification".into(),
            ));
        }
        if sha256_file(&artifact.path)? != artifact.sha256 {
            return Err(AppError::Integrity(
                "previous artifact hash mismatch".into(),
            ));
        }
        let temporary = destination.with_extension("rollback-pending");
        fs::copy(&artifact.path, &temporary)?;
        let displaced = destination.with_extension("rollback-displaced");
        if displaced.exists() {
            fs::remove_file(&displaced)?;
        }
        if destination.exists() {
            fs::rename(destination, &displaced)?;
        }
        if let Err(error) = fs::rename(&temporary, destination) {
            if displaced.exists() {
                let _ = fs::rename(&displaced, destination);
            }
            return Err(error.into());
        }
        if displaced.exists() {
            fs::remove_file(displaced)?;
        }
        Ok(())
    }
}
pub struct JournalStore {
    path: PathBuf,
}
impl JournalStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load(&self) -> Result<Option<UpdateJournal>, AppError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&self.path)?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| AppError::Integrity("update recovery journal is invalid".into()))
    }
    pub fn save(&self, value: &UpdateJournal) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?
        }
        let temp = self.path.with_extension("journal-pending");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(value).map_err(|_| AppError::Internal)?,
        )?;
        fs::rename(temp, &self.path)?;
        Ok(())
    }
    pub fn stage_download(
        &self,
        current_version: &str,
        target_version: &str,
        downloaded_sha256: &str,
        previous_artifact: SignedArtifact,
        database: DatabaseRecovery,
    ) -> Result<(), AppError> {
        if downloaded_sha256.len() != 64
            || !downloaded_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(AppError::Validation(
                "downloaded artifact SHA-256 is invalid".into(),
            ));
        }
        self.save(&UpdateJournal {
            current_version: current_version.into(),
            target_version: target_version.into(),
            downloaded_sha256: downloaded_sha256.to_ascii_lowercase(),
            previous_artifact,
            database,
            phase: Phase::Downloaded,
            health_attempts: 0,
        })
    }
    pub fn mark_pending_install(&self) -> Result<(), AppError> {
        let mut value = self
            .load()?
            .ok_or_else(|| AppError::NotFound("update journal".into()))?;
        if value.phase != Phase::Downloaded {
            return Err(AppError::Conflict("update is not downloaded".into()));
        }
        value.phase = Phase::PendingInstall;
        self.save(&value)
    }
    pub fn mark_awaiting_health(&self) -> Result<(), AppError> {
        let mut value = self
            .load()?
            .ok_or_else(|| AppError::NotFound("update journal".into()))?;
        if value.phase != Phase::PendingInstall {
            return Err(AppError::Conflict(
                "update is not pending installation".into(),
            ));
        }
        value.phase = Phase::AwaitingHealth;
        self.save(&value)
    }
    pub fn mark_install_failed(&self) -> Result<(), AppError> {
        let mut value = self
            .load()?
            .ok_or_else(|| AppError::NotFound("update journal".into()))?;
        if !matches!(value.phase, Phase::PendingInstall | Phase::AwaitingHealth) {
            return Err(AppError::Conflict("update is not being installed".into()));
        }
        value.phase = Phase::Downloaded;
        value.health_attempts = 0;
        self.save(&value)
    }
    pub fn startup_decision(&self, running_version: &str) -> Result<StartupDecision, AppError> {
        let Some(mut value) = self.load()? else {
            return Ok(StartupDecision::None);
        };
        if value.phase != Phase::AwaitingHealth {
            return Ok(if value.phase == Phase::RecoveryRequired {
                StartupDecision::Recover(Box::new(value))
            } else {
                StartupDecision::None
            });
        }
        if running_version != value.target_version {
            value.phase = Phase::RecoveryRequired;
            self.save(&value)?;
            return Ok(StartupDecision::Recover(Box::new(value)));
        }
        if value.health_attempts == 0 {
            value.health_attempts = 1;
            self.save(&value)?;
            Ok(StartupDecision::ProceedHealthCheck)
        } else {
            value.phase = Phase::RecoveryRequired;
            self.save(&value)?;
            Ok(StartupDecision::Recover(Box::new(value)))
        }
    }
    pub fn commit_health(&self, running_version: &str) -> Result<(), AppError> {
        let mut value = self
            .load()?
            .ok_or_else(|| AppError::NotFound("update journal".into()))?;
        if value.phase != Phase::AwaitingHealth || value.target_version != running_version {
            return Err(AppError::Conflict(
                "no matching update health check is pending".into(),
            ));
        }
        value.phase = Phase::Healthy;
        self.save(&value)
    }
    pub fn recover(
        &self,
        journal: &UpdateJournal,
        installer: &dyn RecoveryInstaller,
        installed_binary: &Path,
    ) -> Result<(), AppError> {
        if current_schema_version(&journal.database.database_path)?
            > journal.database.previous_max_schema_version
        {
            fs::copy(
                &journal.database.backup_path,
                &journal.database.database_path,
            )?;
        }
        installer.restore(&journal.previous_artifact, installed_binary)?;
        let mut completed = journal.clone();
        completed.phase = Phase::RolledBack;
        self.save(&completed)
    }
}
pub fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut digest = Sha256::new();
    digest.update(fs::read(path)?);
    Ok(format!("{:x}", digest.finalize()))
}
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}
fn current_schema_version(path: &Path) -> Result<i64, AppError> {
    let c = rusqlite::Connection::open(path)?;
    c.query_row(
        "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_survives_restart_commits_health_or_restores_real_files_and_database() {
        let dir = tempfile::tempdir().unwrap();
        let journal_path = dir.path().join("update.json");
        let store = JournalStore::new(&journal_path);
        let old = dir.path().join("old.exe");
        let installed = dir.path().join("app.exe");
        fs::write(&old, b"signed-old-binary").unwrap();
        fs::write(&installed, b"new-binary").unwrap();
        let db = dir.path().join("app.db");
        let backup = dir.path().join("app-v8.db");
        for (path, version) in [(&db, 9), (&backup, 8)] {
            let c = rusqlite::Connection::open(path).unwrap();
            c.execute("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT,applied_at TEXT)",[]).unwrap();
            c.execute(
                "INSERT INTO schema_migrations VALUES(?1,'x','now')",
                [version],
            )
            .unwrap();
        }
        let artifact = SignedArtifact {
            version: "1.0".into(),
            path: old.clone(),
            sha256: sha256_file(&old).unwrap(),
            signature_verified: true,
        };
        let recovery = DatabaseRecovery {
            database_path: db.clone(),
            backup_path: backup.clone(),
            previous_max_schema_version: 8,
        };
        store
            .stage_download("1.0", "2.0", &"a".repeat(64), artifact, recovery)
            .unwrap();
        store.mark_pending_install().unwrap();
        store.mark_awaiting_health().unwrap();
        assert_eq!(
            JournalStore::new(&journal_path)
                .startup_decision("2.0")
                .unwrap(),
            StartupDecision::ProceedHealthCheck
        );
        let plan = match JournalStore::new(&journal_path)
            .startup_decision("2.0")
            .unwrap()
        {
            StartupDecision::Recover(v) => v,
            _ => panic!("crashed first health must require recovery"),
        };
        store
            .recover(&plan, &VerifiedFileInstaller, &installed)
            .unwrap();
        assert_eq!(fs::read(&installed).unwrap(), b"signed-old-binary");
        assert_eq!(current_schema_version(&db).unwrap(), 8);
        assert_eq!(store.load().unwrap().unwrap().phase, Phase::RolledBack)
    }
    #[test]
    fn healthy_first_start_commits_without_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("old");
        fs::write(&old, b"old").unwrap();
        let db = dir.path().join("db");
        let c = rusqlite::Connection::open(&db).unwrap();
        c.execute(
            "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY)",
            [],
        )
        .unwrap();
        drop(c);
        let store = JournalStore::new(dir.path().join("journal"));
        store
            .stage_download(
                "1",
                "2",
                &"b".repeat(64),
                SignedArtifact {
                    version: "1".into(),
                    path: old.clone(),
                    sha256: sha256_file(&old).unwrap(),
                    signature_verified: true,
                },
                DatabaseRecovery {
                    database_path: db.clone(),
                    backup_path: db,
                    previous_max_schema_version: 0,
                },
            )
            .unwrap();
        store.mark_pending_install().unwrap();
        store.mark_awaiting_health().unwrap();
        assert_eq!(
            store.startup_decision("2").unwrap(),
            StartupDecision::ProceedHealthCheck
        );
        store.commit_health("2").unwrap();
        assert_eq!(store.load().unwrap().unwrap().phase, Phase::Healthy)
    }
    #[test]
    fn unsigned_previous_artifact_never_claims_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = SignedArtifact {
            version: "1".into(),
            path: dir.path().join("old"),
            sha256: "0".repeat(64),
            signature_verified: false,
        };
        assert!(VerifiedFileInstaller
            .restore(&artifact, &dir.path().join("app"))
            .is_err())
    }
}
