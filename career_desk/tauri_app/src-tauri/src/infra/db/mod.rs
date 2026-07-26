mod migration;

use crate::error::AppError;
use fs2::FileExt;
use migration::MIGRATIONS;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RUNTIME_DB_PATH: OnceLock<PathBuf> = OnceLock::new();
const BACKUP_RETENTION: usize = 5;
type ForeignKeyShape = (String, String, String, String, String);

struct TemporaryDatabaseFiles(PathBuf);
impl Drop for TemporaryDatabaseFiles {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        let _ = fs::remove_file(self.0.with_extension("db.lock"));
        let _ = fs::remove_file(self.0.with_extension("db-wal"));
        let _ = fs::remove_file(self.0.with_extension("db-shm"));
    }
}

fn unique_temporary_database(parent: &Path, purpose: &str) -> Result<PathBuf, AppError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    loop {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = parent.join(format!(
            ".{purpose}-{nanos}-{}-{}.db",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
}

pub const LEGACY_TABLES: [&str; 9] = [
    "experiences",
    "personas",
    "role_experience_weights",
    "skill_nodes",
    "job_descs",
    "job_matches",
    "job_match_experience_reframes",
    "learning_paths",
    "uploaded_files",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaProbe {
    pub existing_tables: Vec<String>,
    pub missing_legacy_tables: Vec<String>,
    pub is_empty: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub path: String,
    pub schema_version: i64,
    pub backup_path: Option<String>,
    pub legacy_table_count: usize,
}

pub struct Database {
    connection: Connection,
    _lock: File,
    status: DatabaseStatus,
}

impl Database {
    pub fn open_and_migrate(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("db.lock"))?;
        lock.try_lock_exclusive()
            .map_err(|_| AppError::DatabaseLocked)?;
        let existed = path.exists() && fs::metadata(path)?.len() > 0;
        let mut connection = configured_connection(path)?;
        integrity_check(&connection)?;
        let before = probe_schema(&connection)?;
        if existed && !before.is_empty && !before.missing_legacy_tables.is_empty() {
            return Err(AppError::IncompatibleSchema(before.missing_legacy_tables));
        }
        let needs_migration = needs_migration(&connection)?;
        let backup_path = if existed && needs_migration {
            Some(create_backup(&connection, path)?)
        } else {
            None
        };
        if let Err(error) = apply_migrations(&mut connection) {
            drop(connection);
            if let Some(backup) = backup_path.as_ref() {
                fs::copy(backup, path)?;
                integrity_check(&configured_connection(path)?)?;
            }
            return Err(AppError::Unavailable(format!(
                "database migration failed; the pre-migration backup was restored: {error}"
            )));
        }
        integrity_check(&connection)?;
        let after = probe_schema(&connection)?;
        if !after.missing_legacy_tables.is_empty() {
            return Err(AppError::IncompatibleSchema(after.missing_legacy_tables));
        }
        let version = current_version(&connection)?;
        let legacy_table_count = after
            .existing_tables
            .iter()
            .filter(|t| LEGACY_TABLES.contains(&t.as_str()))
            .count();
        Ok(Self {
            connection,
            _lock: lock,
            status: DatabaseStatus {
                path: path.to_string_lossy().into_owned(),
                schema_version: version,
                backup_path: backup_path.map(|p| p.to_string_lossy().into_owned()),
                legacy_table_count,
            },
        })
    }
    pub fn status(&self) -> &DatabaseStatus {
        &self.status
    }
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

fn configured_connection(path: &Path) -> Result<Connection, AppError> {
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(connection)
}

/// Pins the database selected and validated during bootstrap. Commands must use
/// `open_runtime_connection` rather than deriving paths independently.
pub fn configure_runtime_path(path: impl AsRef<Path>) -> Result<(), AppError> {
    let path = path.as_ref().to_path_buf();
    if let Some(existing) = RUNTIME_DB_PATH.get() {
        return if existing == &path {
            Ok(())
        } else {
            Err(AppError::Conflict(
                "runtime database path is already configured".into(),
            ))
        };
    }
    RUNTIME_DB_PATH
        .set(path)
        .map_err(|_| AppError::Conflict("runtime database path is already configured".into()))
}

pub fn open_runtime_connection() -> Result<Connection, AppError> {
    #[cfg(test)]
    if let Some(path) = std::env::var_os("CAREERCRAFT_DB_PATH") {
        return configured_connection(Path::new(&path));
    }
    let path = RUNTIME_DB_PATH
        .get()
        .ok_or_else(|| AppError::Unavailable("database runtime is not initialized".into()))?;
    configured_connection(path)
}

pub(crate) fn latest_schema_version() -> i64 {
    MIGRATIONS.last().map_or(0, |migration| migration.version)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
}

fn runtime_path() -> Result<&'static Path, AppError> {
    RUNTIME_DB_PATH
        .get()
        .map(PathBuf::as_path)
        .ok_or_else(|| AppError::Unavailable("database runtime is not initialized".into()))
}

pub fn list_backups() -> Result<Vec<BackupInfo>, AppError> {
    let dir = runtime_path()?
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut values = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_owned();
            if !name.starts_with("career-") || !name.ends_with(".db") {
                return None;
            }
            Some(BackupInfo {
                size: entry.metadata().ok()?.len(),
                name,
                path: path.to_string_lossy().into_owned(),
            })
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(values)
}

pub fn create_runtime_backup() -> Result<BackupInfo, AppError> {
    let path = create_backup(&open_runtime_connection()?, runtime_path()?)?;
    Ok(BackupInfo {
        size: fs::metadata(&path)?.len(),
        name: path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or_default()
            .to_owned(),
        path: path.to_string_lossy().into_owned(),
    })
}

pub fn stage_restore_backup(name: &str) -> Result<PathBuf, AppError> {
    if name.contains(['/', '\\']) || !name.starts_with("career-") || !name.ends_with(".db") {
        return Err(AppError::Validation("invalid backup name".into()));
    }
    let db = runtime_path()?;
    let source = db
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
        .join(name);
    integrity_check(&configured_connection(&source)?)?;
    let pending = db.with_extension("restore-pending.db");
    fs::copy(source, &pending)?;
    integrity_check(&configured_connection(&pending)?)?;
    Ok(pending)
}

pub fn apply_pending_restore(path: &Path) -> Result<bool, AppError> {
    apply_pending_restore_with(path, |_| Ok(()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreStep {
    CurrentDisplaced,
    CandidateInstalled,
}

fn apply_pending_restore_with(
    path: &Path,
    mut checkpoint: impl FnMut(RestoreStep) -> Result<(), AppError>,
) -> Result<bool, AppError> {
    let pending = path.with_extension("restore-pending.db");
    if !pending.exists() {
        return Ok(false);
    }
    validate_restore_candidate(&pending)?;
    let displaced = unique_restore_safety_path(path)?;
    let had_current = path.exists();
    if had_current {
        fs::rename(path, &displaced)?;
        if let Err(error) = checkpoint(RestoreStep::CurrentDisplaced) {
            fs::rename(&displaced, path)?;
            return Err(error);
        }
    }
    if let Err(error) = fs::rename(&pending, path) {
        if had_current {
            fs::rename(&displaced, path)?;
        }
        return Err(error.into());
    }
    if let Err(error) = checkpoint(RestoreStep::CandidateInstalled) {
        fs::rename(path, &pending)?;
        if had_current {
            fs::rename(&displaced, path)?;
        }
        return Err(error);
    }
    if let Err(error) = validate_restore_candidate(path) {
        let _ = fs::rename(path, &pending);
        if had_current {
            let _ = fs::rename(&displaced, path);
        }
        return Err(error);
    }
    if had_current {
        prune_restore_safety(
            displaced.parent().unwrap_or_else(|| Path::new(".")),
            BACKUP_RETENTION,
        )?;
    }
    Ok(true)
}

fn unique_restore_safety_path(database: &Path) -> Result<PathBuf, AppError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = database
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    fs::create_dir_all(&directory)?;
    loop {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let candidate = directory.join(format!(
            "career-pre-restore-{nanos}-{}-{}.db",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
}

fn prune_restore_safety(directory: &Path, retain: usize) -> Result<(), AppError> {
    let mut values = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|name| {
                    name.starts_with("career-pre-restore-") && name.ends_with(".db")
                })
        })
        .collect::<Vec<_>>();
    values.sort();
    let remove = values.len().saturating_sub(retain);
    for path in values.into_iter().take(remove) {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Validates an imported database without modifying it, then migrates and opens
/// a disposable sibling copy through the same path used by the real runtime.
pub(crate) fn validate_restore_candidate(path: &Path) -> Result<(), AppError> {
    let readonly = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    integrity_check(&readonly)?;
    let probe = probe_schema(&readonly)?;
    if probe.is_empty || !probe.missing_legacy_tables.is_empty() {
        return Err(AppError::IncompatibleSchema(probe.missing_legacy_tables));
    }
    let version = current_version(&readonly)?;
    if !(1..=latest_schema_version()).contains(&version) {
        return Err(AppError::IncompatibleSchema(vec![format!(
            "unsupported migration version {version}"
        )]));
    }
    let versions = readonly
        .prepare("SELECT version FROM schema_migrations ORDER BY version")?
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if versions != (1..=version).collect::<Vec<_>>() {
        return Err(AppError::IncompatibleSchema(vec![
            "schema migration history is not continuous".into(),
        ]));
    }
    let mut required_shapes = vec![
        ("schema_migrations", &["version", "name", "applied_at"][..]),
        (
            "experiences",
            &[
                "id",
                "user_id",
                "type",
                "title",
                "raw_description",
                "status",
                "version",
            ][..],
        ),
    ];
    if version >= 2 {
        required_shapes.push((
            "provider_configs",
            &[
                "name",
                "base_url",
                "default_model",
                "credential_target",
                "enabled",
            ][..],
        ));
    }
    if version >= 5 {
        required_shapes.push(("experiences", &["industry_tags", "education_level"][..]));
    }
    if version >= 9 {
        required_shapes.push((
            "experience_revisions",
            &[
                "experience_id",
                "revision",
                "source",
                "snapshot_json",
                "deleted",
                "created_at",
            ][..],
        ));
    }
    for (table, required) in required_shapes {
        let pragma = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
        let columns = readonly
            .prepare(&pragma)?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<std::collections::HashSet<_>, _>>()?;
        let missing = required
            .iter()
            .filter(|column| !columns.contains(**column))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(AppError::IncompatibleSchema(vec![format!(
                "{table} missing columns: {}",
                missing.join(",")
            )]));
        }
    }
    let revision_index: bool = readonly.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_experience_revisions_recent' AND tbl_name='experience_revisions')",
        [], |row| row.get(0),
    )?;
    if version >= 9 && !revision_index {
        return Err(AppError::IncompatibleSchema(vec![
            "missing experience revision index".into(),
        ]));
    }
    let role_foreign_keys: i64 = readonly.query_row(
        "SELECT COUNT(*) FROM pragma_foreign_key_list('role_experience_weights') WHERE \"table\" IN ('personas','experiences')",
        [], |row| row.get(0),
    )?;
    if role_foreign_keys != 2 {
        return Err(AppError::IncompatibleSchema(vec![
            "role experience foreign keys are missing".into(),
        ]));
    }
    drop(readonly);

    let smoke = path.with_extension(format!("restore-smoke-{}.db", std::process::id()));
    if smoke.exists() {
        return Err(AppError::Conflict(
            "restore smoke path already exists".into(),
        ));
    }
    fs::copy(path, &smoke)?;
    let result = (|| {
        let database = Database::open_and_migrate(&smoke)?;
        integrity_check(database.connection())?;
        let reference = unique_temporary_database(
            path.parent().unwrap_or_else(|| Path::new(".")),
            "restore-reference",
        )?;
        let _reference_cleanup = TemporaryDatabaseFiles(reference.clone());
        let expected = Database::open_and_migrate(&reference)?;
        assert_required_schema(database.connection(), expected.connection())?;
        drop(expected);
        database
            .connection()
            .query_row(
                "SELECT id,title FROM experiences ORDER BY id LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        database
            .connection()
            .query_row(
                "SELECT name,enabled FROM provider_configs ORDER BY name LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
            )
            .optional()?;
        Ok(())
    })();
    let _ = fs::remove_file(&smoke);
    let _ = fs::remove_file(smoke.with_extension("db.lock"));
    let _ = fs::remove_file(smoke.with_extension("db-wal"));
    let _ = fs::remove_file(smoke.with_extension("db-shm"));
    result
}

fn assert_required_schema(candidate: &Connection, expected: &Connection) -> Result<(), AppError> {
    fn names(connection: &Connection, kind: &str) -> Result<Vec<String>, AppError> {
        Ok(connection
            .prepare("SELECT name FROM sqlite_master WHERE type=?1 AND name NOT LIKE 'sqlite_%' ORDER BY name")?
            .query_map([kind], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }
    for kind in ["table", "index", "trigger"] {
        let actual = names(candidate, kind)?;
        for name in names(expected, kind)? {
            if !actual.contains(&name) {
                return Err(AppError::IncompatibleSchema(vec![format!(
                    "missing required {kind} {name}"
                )]));
            }
        }
    }
    let normalized_sql = |connection: &Connection, name: &str| -> Result<String, AppError> {
        let sql: String = connection.query_row(
            "SELECT COALESCE(sql,'') FROM sqlite_master WHERE type='trigger' AND name=?1",
            [name],
            |row| row.get(0),
        )?;
        Ok(sql
            .to_ascii_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "))
    };
    for trigger in names(expected, "trigger")? {
        if normalized_sql(candidate, &trigger)? != normalized_sql(expected, &trigger)? {
            return Err(AppError::IncompatibleSchema(vec![format!(
                "required trigger differs: {trigger}"
            )]));
        }
    }
    for table in names(expected, "table")? {
        let escaped = table.replace('\'', "''");
        let columns =
            |connection: &Connection| -> Result<Vec<(String, String, i64, i64)>, AppError> {
                Ok(connection.prepare(&format!("SELECT name,type,\"notnull\",pk FROM pragma_table_info('{escaped}') ORDER BY cid"))?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
                .collect::<Result<Vec<_>, _>>()?)
            };
        if columns(candidate)? != columns(expected)? {
            return Err(AppError::IncompatibleSchema(vec![format!(
                "required columns differ for {table}"
            )]));
        }
        let fks = |connection: &Connection| -> Result<Vec<ForeignKeyShape>, AppError> {
            Ok(connection.prepare(&format!("SELECT \"table\",\"from\",\"to\",on_update,on_delete FROM pragma_foreign_key_list('{escaped}') ORDER BY id,seq"))?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?
                .collect::<Result<Vec<_>, _>>()?)
        };
        if fks(candidate)? != fks(expected)? {
            return Err(AppError::IncompatibleSchema(vec![format!(
                "required foreign keys differ for {table}"
            )]));
        }
    }
    Ok(())
}

pub fn probe_schema(connection: &Connection) -> Result<SchemaProbe, AppError> {
    let mut stmt = connection.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?;
    let existing_tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    let missing_legacy_tables = LEGACY_TABLES
        .iter()
        .filter(|name| !existing_tables.iter().any(|table| table == **name))
        .map(|s| s.to_string())
        .collect();
    Ok(SchemaProbe {
        is_empty: existing_tables.is_empty(),
        existing_tables,
        missing_legacy_tables,
    })
}

pub fn integrity_check(connection: &Connection) -> Result<(), AppError> {
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(AppError::Integrity(integrity));
    }
    let fk_errors: i64 =
        connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if fk_errors != 0 {
        return Err(AppError::Integrity(format!(
            "{fk_errors} foreign key violation(s)"
        )));
    }
    Ok(())
}

fn create_backup(connection: &Connection, original: &Path) -> Result<PathBuf, AppError> {
    let backup_dir = original
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    fs::create_dir_all(&backup_dir)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let backup = backup_dir.join(format!("career-{timestamp}.db"));
    connection.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])?;
    integrity_check(&Connection::open(&backup)?)?;
    prune_backups(&backup_dir, BACKUP_RETENTION)?;
    Ok(backup)
}

fn prune_backups(dir: &Path, retain: usize) -> Result<(), AppError> {
    let mut backups = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("career-") && name.ends_with(".db"))
        })
        .collect::<Vec<_>>();
    backups.sort();
    let remove_count = backups.len().saturating_sub(retain);
    for path in backups.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn needs_migration(connection: &Connection) -> Result<bool, AppError> {
    let has_table: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
        [],
        |row| row.get(0),
    )?;
    if !has_table {
        return Ok(true);
    }
    Ok(current_version(connection)? < MIGRATIONS.last().map_or(0, |m| m.version))
}

fn apply_migrations(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction()?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL)")?;
    for migration in MIGRATIONS {
        let applied: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=?1)",
            [migration.version],
            |row| row.get(0),
        )?;
        if applied {
            continue;
        }
        transaction.execute_batch(migration.sql)?;
        let applied_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        transaction.execute(
            "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,?3)",
            rusqlite::params![migration.version, migration.name, applied_at],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn current_version(connection: &Connection) -> Result<i64, AppError> {
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn create_v6(path: &Path) {
        let c = configured_connection(path).unwrap();
        c.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at TEXT NOT NULL)").unwrap();
        for migration in &migration::MIGRATIONS[..6] {
            c.execute_batch(migration.sql).unwrap();
            c.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,'now')",
                rusqlite::params![migration.version, migration.name],
            )
            .unwrap();
        }
    }
    fn create_v7(path: &Path) {
        let c = configured_connection(path).unwrap();
        c.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at TEXT NOT NULL)").unwrap();
        for migration in &migration::MIGRATIONS[..7] {
            c.execute_batch(migration.sql).unwrap();
            c.execute(
                "INSERT INTO schema_migrations(version,name,applied_at) VALUES(?1,?2,'now')",
                rusqlite::params![migration.version, migration.name],
            )
            .unwrap();
        }
    }
    #[test]
    fn creates_all_legacy_tables_and_records_migration() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_and_migrate(dir.path().join("career.db")).unwrap();
        assert_eq!(db.status().schema_version, 10);
        assert_eq!(db.status().legacy_table_count, 9);
        assert!(probe_schema(db.connection())
            .unwrap()
            .missing_legacy_tables
            .is_empty());
        let columns = db
            .connection()
            .prepare("SELECT name FROM pragma_table_info('experiences')")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            columns.contains(&"industry_tags".into())
                && columns.contains(&"education_level".into())
        );
        let defaults: (String, String) = db
            .connection()
            .query_row(
                "SELECT industry_tags,education_levels FROM job_descs LIMIT 0",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or(("[]".into(), "[]".into()));
        assert_eq!(defaults, ("[]".into(), "[]".into()));
    }
    #[test]
    fn migration9_backfills_existing_experience_revision_without_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v8.db");
        {
            let c = configured_connection(&path).unwrap();
            c.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at TEXT NOT NULL)").unwrap();
            for migration in &migration::MIGRATIONS[..8] {
                c.execute_batch(migration.sql).unwrap();
                c.execute(
                    "INSERT INTO schema_migrations VALUES(?1,?2,'now')",
                    rusqlite::params![migration.version, migration.name],
                )
                .unwrap();
            }
            c.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('legacy','u','work','旧经历','原文','draft',7)", []).unwrap();
        }
        let db = Database::open_and_migrate(&path).unwrap();
        let (revision, source, title): (i64, String, String) = db.connection().query_row(
            "SELECT revision,source,json_extract(snapshot_json,'$.title') FROM experience_revisions WHERE experience_id='legacy'", [],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).unwrap();
        assert_eq!(
            (revision, source.as_str(), title.as_str()),
            (7, "migration", "旧经历")
        );
    }
    #[test]
    fn migration10_rejects_direct_revision_update_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_and_migrate(dir.path().join("append-only.db")).unwrap();
        db.connection().execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('e','u','work','T','R','draft',1)", []).unwrap();
        db.connection().execute("INSERT INTO experience_revisions(experience_id,revision,source,snapshot_json) VALUES('e',1,'create','{}')", []).unwrap();
        assert!(db
            .connection()
            .execute(
                "UPDATE experience_revisions SET source='update' WHERE experience_id='e'",
                []
            )
            .is_err());
        assert!(db
            .connection()
            .execute(
                "DELETE FROM experience_revisions WHERE experience_id='e'",
                []
            )
            .is_err());
        assert_eq!(
            db.connection()
                .query_row(
                    "SELECT COUNT(*) FROM experience_revisions WHERE experience_id='e'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }
    #[test]
    fn current_database_does_not_create_redundant_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        {
            let db = Database::open_and_migrate(&path).unwrap();
            db.connection().execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('e','default','work','title','raw','draft',1)", []).unwrap();
        }
        let db = Database::open_and_migrate(&path).unwrap();
        assert!(db.status().backup_path.is_none());
        let count: i64 = db
            .connection()
            .query_row("SELECT count(*) FROM experiences", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
    #[test]
    fn rejects_partial_schema_without_mutating_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        Connection::open(&path)
            .unwrap()
            .execute("CREATE TABLE experiences(id TEXT PRIMARY KEY)", [])
            .unwrap();
        let error = match Database::open_and_migrate(&path) {
            Ok(_) => panic!("partial schema accepted"),
            Err(e) => e,
        };
        assert!(matches!(error, AppError::IncompatibleSchema(_)));
        assert!(!probe_schema(&Connection::open(path).unwrap())
            .unwrap()
            .existing_tables
            .contains(&"schema_migrations".to_string()));
    }
    #[test]
    fn backup_retention_keeps_newest_five() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..7 {
            fs::write(dir.path().join(format!("career-{index:02}.db")), b"x").unwrap();
        }
        prune_backups(dir.path(), 5).unwrap();
        let names = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(names.len(), 5);
        assert!(!names.contains(&"career-00.db".to_string()));
        assert!(!names.contains(&"career-01.db".to_string()));
    }
    #[test]
    fn pending_restore_is_validated_and_applied_before_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        {
            let db = Database::open_and_migrate(&path).unwrap();
            db.connection().execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('old','default','work','old','old','draft',1)",[]).unwrap();
        }
        let pending = path.with_extension("restore-pending.db");
        {
            let db = Database::open_and_migrate(&pending).unwrap();
            db.connection().execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('restored','default','work','new','new','draft',1)",[]).unwrap();
        }
        assert!(apply_pending_restore(&path).unwrap());
        let c = configured_connection(&path).unwrap();
        let restored: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM experiences WHERE id='restored')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(restored);
        assert_eq!(restore_safety_files(dir.path()).len(), 1);
    }
    #[test]
    fn restore_failure_after_displacement_restores_current_and_keeps_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        {
            let db = Database::open_and_migrate(&path).unwrap();
            db.connection().execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('old','default','work','old','old','draft',1)",[]).unwrap();
        }
        let pending = path.with_extension("restore-pending.db");
        {
            let db = Database::open_and_migrate(&pending).unwrap();
            db.connection().execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('new','default','work','new','new','draft',1)",[]).unwrap();
        }
        let error = apply_pending_restore_with(&path, |step| {
            if step == RestoreStep::CurrentDisplaced {
                Err(AppError::Unavailable("injected restore failure".into()))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert!(matches!(error, AppError::Unavailable(_)));
        assert!(pending.exists());
        let current = configured_connection(&path).unwrap();
        assert!(current
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM experiences WHERE id='old')",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap());
    }
    #[test]
    fn restore_failure_after_candidate_install_rolls_both_names_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        drop(Database::open_and_migrate(&path).unwrap());
        let pending = path.with_extension("restore-pending.db");
        drop(Database::open_and_migrate(&pending).unwrap());
        let result = apply_pending_restore_with(&path, |step| {
            if step == RestoreStep::CandidateInstalled {
                Err(AppError::Unavailable(
                    "injected post-install failure".into(),
                ))
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert!(path.exists() && pending.exists());
        assert!(restore_safety_files(dir.path()).is_empty());
        validate_restore_candidate(&path).unwrap();
        validate_restore_candidate(&pending).unwrap();
    }
    #[test]
    fn restore_validation_rejects_fake_migration_table_database() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("fake.db");
        Connection::open(&fake).unwrap().execute_batch(
            "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at TEXT NOT NULL); INSERT INTO schema_migrations VALUES(1,'fake','now')",
        ).unwrap();
        assert!(matches!(
            validate_restore_candidate(&fake),
            Err(AppError::IncompatibleSchema(_))
        ));
    }
    #[test]
    fn restore_validation_rejects_missing_or_semantically_tampered_append_only_trigger() {
        let dir = tempfile::tempdir().unwrap();
        for tampered in [false, true] {
            let path = dir.path().join(format!("trigger-{tampered}.db"));
            {
                let db = Database::open_and_migrate(&path).unwrap();
                db.connection()
                    .execute("DROP TRIGGER experience_revisions_no_delete", [])
                    .unwrap();
                if tampered {
                    db.connection().execute_batch(
                        "CREATE TRIGGER experience_revisions_no_delete BEFORE DELETE ON experience_revisions BEGIN SELECT 1; END",
                    ).unwrap();
                }
            }
            assert!(matches!(
                validate_restore_candidate(&path),
                Err(AppError::IncompatibleSchema(_))
            ));
        }
    }
    #[test]
    fn failed_deep_validation_cleans_reference_and_next_validation_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("candidate.db");
        {
            let db = Database::open_and_migrate(&path).unwrap();
            db.connection()
                .execute("DROP TRIGGER experience_revisions_no_delete", [])
                .unwrap();
            db.connection().execute_batch(
                "CREATE TRIGGER experience_revisions_no_delete BEFORE DELETE ON experience_revisions BEGIN SELECT 1; END",
            ).unwrap();
        }
        assert!(validate_restore_candidate(&path).is_err());
        {
            let connection = configured_connection(&path).unwrap();
            connection
                .execute("DROP TRIGGER experience_revisions_no_delete", [])
                .unwrap();
            connection
                .execute_batch(
                    "CREATE TRIGGER experience_revisions_no_delete
                 BEFORE DELETE ON experience_revisions
                 BEGIN
                   SELECT RAISE(ABORT, 'experience revisions are append-only');
                 END;",
                )
                .unwrap();
        }
        validate_restore_candidate(&path).unwrap();
        assert!(fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .contains("restore-reference")));
    }
    #[test]
    fn repeated_restores_use_unique_safety_files_and_retain_newest_five() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        drop(Database::open_and_migrate(&path).unwrap());
        for index in 0..7 {
            let pending = path.with_extension("restore-pending.db");
            {
                let db = Database::open_and_migrate(&pending).unwrap();
                db.connection().execute(
                    "INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES(?1,'u','work','T','R','draft',1)",
                    [format!("restore-{index}")],
                ).unwrap();
            }
            assert!(apply_pending_restore(&path).unwrap());
        }
        let safety = restore_safety_files(dir.path());
        assert_eq!(safety.len(), BACKUP_RETENTION);
        let current = configured_connection(&path).unwrap();
        assert!(current
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM experiences WHERE id='restore-6')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .unwrap());
    }

    fn restore_safety_files(root: &Path) -> Vec<PathBuf> {
        let directory = root.join("backups");
        if !directory.exists() {
            return Vec::new();
        }
        fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|v| v.to_str())
                    .is_some_and(|name| {
                        name.starts_with("career-pre-restore-") && name.ends_with(".db")
                    })
            })
            .collect()
    }
    #[test]
    fn no_pending_restore_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!apply_pending_restore(&dir.path().join("career.db")).unwrap());
    }
    #[test]
    fn integrity_check_reports_foreign_key_damage() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!(
            "../../../../contracts/db/legacy_schema_v1.sql"
        ))
        .unwrap();
        c.pragma_update(None, "foreign_keys", "OFF").unwrap();
        c.execute("INSERT INTO role_experience_weights(id,persona_id,experience_id,relevance_score,user_overridden) VALUES('w','missing-p','missing-e',0,0)",[]).unwrap();
        assert!(
            matches!(integrity_check(&c),Err(AppError::Integrity(message)) if message.contains("foreign key"))
        );
    }
    #[test]
    fn failed_migration_restores_pre_migration_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(include_str!(
                "../../../../contracts/db/legacy_schema_v1.sql"
            ))
            .unwrap();
            c.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at TEXT NOT NULL);INSERT INTO schema_migrations VALUES(1,'legacy','now');INSERT INTO schema_migrations VALUES(2,'providers','now');CREATE TABLE resume_versions(bad TEXT);").unwrap();
        }
        let error = Database::open_and_migrate(&path)
            .err()
            .expect("migration must fail");
        assert!(matches!(error, AppError::Unavailable(_)));
        let c = Connection::open(&path).unwrap();
        let bad_exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('resume_versions') WHERE name='bad')",[],|r|r.get(0)).unwrap();
        assert!(bad_exists);
        let version: i64 = c
            .query_row("SELECT max(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 2);
    }
    #[test]
    fn migration7_backfills_once_survives_restart_and_cascades_with_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v6.db");
        create_v6(&path);
        {
            let c = configured_connection(&path).unwrap();
            c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','P',1,'{}','[]',5)",[]).unwrap();
            c.execute("INSERT INTO job_descs(id,raw_text) VALUES('j','job')", [])
                .unwrap();
            c.execute("INSERT INTO job_matches(id,persona_id,job_desc_id,tracking_status) VALUES('m','p','j','applied')",[]).unwrap();
        }
        let db = Database::open_and_migrate(&path).unwrap();
        let event: (Option<String>, String) = db
            .connection()
            .query_row(
                "SELECT from_status,to_status FROM job_status_events WHERE match_id='m'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(event, (None, "applied".into()));
        drop(db);
        let reopened = Database::open_and_migrate(&path).unwrap();
        assert_eq!(
            reopened
                .connection()
                .query_row(
                    "SELECT COUNT(*) FROM job_status_events WHERE match_id='m'",
                    [],
                    |r| r.get::<_, u32>(0)
                )
                .unwrap(),
            1
        );
        reopened
            .connection()
            .execute("DELETE FROM job_matches WHERE id='m'", [])
            .unwrap();
        assert_eq!(
            reopened
                .connection()
                .query_row("SELECT COUNT(*) FROM job_status_events", [], |r| r
                    .get::<_, u32>(0))
                .unwrap(),
            0
        )
    }
    #[test]
    fn migration7_failure_restores_exact_v6_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken-v7.db");
        create_v6(&path);
        {
            let c = Connection::open(&path).unwrap();
            c.execute("CREATE TABLE job_status_events(bad TEXT)", [])
                .unwrap();
            c.execute("INSERT INTO job_status_events VALUES('sentinel')", [])
                .unwrap();
        }
        let error = Database::open_and_migrate(&path)
            .err()
            .expect("v7 must fail");
        assert!(matches!(error, AppError::Unavailable(_)));
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            c.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            6
        );
        assert_eq!(
            c.query_row("SELECT bad FROM job_status_events", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "sentinel"
        )
    }
    #[test]
    fn migration8_preserves_conversion_as_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v7.db");
        create_v7(&path);
        {
            let c = configured_connection(&path).unwrap();
            c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','P',1,'{}','[]',5)",[]).unwrap();
            c.execute(
                "INSERT INTO learning_paths(id,persona_id,target_gap) VALUES('lp','p','Rust')",
                [],
            )
            .unwrap();
            c.execute("INSERT INTO learning_items(id,path_id,skill_id,title,status,completion_note) VALUES('li','lp','rust','CLI','completed','Built CLI')",[]).unwrap();
            c.execute("INSERT INTO experiences(id,title,type,raw_description,status) VALUES('e','E','project','Built','draft')",[]).unwrap();
            c.execute(
                "INSERT INTO learning_conversions(item_id,experience_id) VALUES('li','e')",
                [],
            )
            .unwrap();
        }
        let db = Database::open_and_migrate(&path).unwrap();
        assert_eq!(db.status().schema_version, 10);
        let row:(String,String,String)=db.connection().query_row("SELECT source_path_id,source_skill_id,completion_note_snapshot FROM learning_conversions",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(row, ("lp".into(), "rust".into(), "Built CLI".into()))
    }
    #[test]
    fn migration8_failure_restores_pre_migration_v7_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken-v8.db");
        create_v7(&path);
        {
            let c = Connection::open(&path).unwrap();
            c.execute(
                "ALTER TABLE job_matches ADD COLUMN version INTEGER NOT NULL DEFAULT 9",
                [],
            )
            .unwrap();
        }
        let error = Database::open_and_migrate(&path)
            .err()
            .expect("v8 must fail");
        assert!(matches!(error, AppError::Unavailable(_)));
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            c.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            7
        );
        assert!(c
            .prepare("SELECT item_id,experience_id FROM learning_conversions")
            .is_ok());
        assert_eq!(
            c.query_row(
                "SELECT dflt_value FROM pragma_table_info('job_matches') WHERE name='version'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "9"
        )
    }
    #[test]
    fn golden_legacy_rows_migrate_1_through_8_without_loss_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("career.db");
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/golden/legacy_rows.json"))
                .unwrap();
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(include_str!(
                "../../../../contracts/db/legacy_schema_v1.sql"
            ))
            .unwrap();
            let e = &fixture["experiences"][0];
            c.execute("INSERT INTO experiences(id,user_id,type,title,organization,start_date,end_date,raw_description,structured_achievements,skills_demonstrated,metrics,status,version,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",rusqlite::params![e["id"].as_str(),e["user_id"].as_str(),e["type"].as_str(),e["title"].as_str(),e["organization"].as_str(),e["start_date"].as_str(),e["end_date"].as_str(),e["raw_description"].as_str(),e["structured_achievements"].to_string(),e["skills_demonstrated"].to_string(),e["metrics"].to_string(),e["status"].as_str(),e["version"].as_i64(),e["created_at"].as_str(),e["updated_at"].as_str()]).unwrap();
            let p = &fixture["personas"][0];
            c.execute("INSERT INTO personas(id,user_id,name,is_default,identity_statement,career_narrative,tone_style,capability_weights,target_job_profiles,max_experiences,preferred_model,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",rusqlite::params![p["id"].as_str(),p["user_id"].as_str(),p["name"].as_str(),p["is_default"].as_bool(),p["identity_statement"].as_str(),p["career_narrative"].as_str(),p["tone_style"].as_str(),p["capability_weights"].to_string(),p["target_job_profiles"].to_string(),p["max_experiences"].as_i64(),p["preferred_model"].as_str(),p["created_at"].as_str(),p["updated_at"].as_str()]).unwrap();
        }
        let backup = {
            let db = Database::open_and_migrate(&path).unwrap();
            assert_eq!(db.status().schema_version, 10);
            integrity_check(db.connection()).unwrap();
            assert_eq!(
                db.connection()
                    .query_row("SELECT raw_description FROM experiences", [], |r| r
                        .get::<_, String>(0))
                    .unwrap(),
                fixture["experiences"][0]["raw_description"]
                    .as_str()
                    .unwrap()
            );
            db.status().backup_path.clone().unwrap()
        };
        fs::copy(&backup, &path).unwrap();
        let restored = Database::open_and_migrate(&path).unwrap();
        integrity_check(restored.connection()).unwrap();
        assert_eq!(
            restored
                .connection()
                .query_row("SELECT count(*) FROM experiences", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
