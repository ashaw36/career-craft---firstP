//! Portable, credential-free backup archives.  Import is deliberately staged:
//! the database is replaced only by `db::apply_pending_restore` on next start.
use crate::error::AppError;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const FORMAT_VERSION: u32 = 1;
#[cfg(not(test))]
const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
#[cfg(test)]
const MAX_ENTRY_BYTES: u64 = 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 300 * 1024 * 1024;
const MAX_ENTRIES: usize = 3;
const DB_NAME: &str = "data/career.db";
const SETTINGS_NAME: &str = "settings/portable.json";
const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableManifest {
    pub format_version: u32,
    pub app_version: String,
    pub schema_version: i64,
    pub created_unix_seconds: u64,
    pub database_sha256: String,
    pub settings_sha256: String,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub manifest: PortableManifest,
    pub record_counts: std::collections::BTreeMap<String, u64>,
    pub app_version: String,
    pub schema_version: i64,
    pub dry_run: bool,
    pub pending_restore: Option<String>,
    pub safety_backup: Option<String>,
}

fn record_counts(path: &Path) -> Result<std::collections::BTreeMap<String, u64>, AppError> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let tables = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut counts = std::collections::BTreeMap::new();
    for table in tables {
        let quoted = table.replace('"', "\"\"");
        let count: u64 =
            connection.query_row(&format!("SELECT COUNT(*) FROM \"{quoted}\""), [], |row| {
                row.get(0)
            })?;
        counts.insert(table, count);
    }
    Ok(counts)
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn zip_error(error: zip::result::ZipError) -> AppError {
    invalid(format!("invalid portable archive: {error}"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unique_path(parent: &Path, stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!(".{stem}-{nanos}-{}", std::process::id()))
}

fn snapshot_database(source: &Path, destination: &Path) -> Result<(), AppError> {
    if destination.exists() {
        return Err(AppError::Conflict(format!(
            "snapshot destination already exists: {}",
            destination.display()
        )));
    }
    let connection = Connection::open(source)?;
    connection.busy_timeout(std::time::Duration::from_secs(10))?;
    connection.execute("VACUUM INTO ?1", [destination.to_string_lossy().as_ref()])?;
    validate_database(destination)
}

fn remove_credential_references(snapshot: &Path) -> Result<(), AppError> {
    let connection = Connection::open(snapshot)?;
    connection.execute("UPDATE provider_configs SET credential_target=''", [])?;
    Ok(())
}

fn schema_version(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn supported_schema() -> i64 {
    super::db::latest_schema_version()
}

fn validate_database(path: &Path) -> Result<(), AppError> {
    super::db::validate_restore_candidate(path)?;
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version = schema_version(&connection)?;
    if version > supported_schema() {
        return Err(invalid(format!(
            "backup schema {version} is newer than supported schema {}",
            supported_schema()
        )));
    }
    Ok(())
}

fn portable_settings(database: &Path) -> Result<Vec<u8>, AppError> {
    let connection = Connection::open(database)?;
    let mut statement = connection.prepare(
        "SELECT name,base_url,default_model,enabled FROM provider_configs ORDER BY name",
    )?;
    let providers = statement
        .query_map([], |row| {
            Ok(json!({
                "name": row.get::<_, String>(0)?,
                "baseUrl": row.get::<_, String>(1)?,
                "defaultModel": row.get::<_, String>(2)?,
                "enabled": row.get::<_, bool>(3)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_vec_pretty(&json!({"providers": providers}))
        .map_err(|error| invalid(error.to_string()))
}

/// Exports an unencrypted archive.  The caller must explicitly acknowledge
/// that anyone who obtains the archive can read its business data.
pub fn export_portable(
    database: &Path,
    destination: &Path,
    acknowledge_unencrypted: bool,
) -> Result<PortableManifest, AppError> {
    if !acknowledge_unencrypted {
        return Err(invalid(
            "unencrypted export requires explicit acknowledgement",
        ));
    }
    if destination.exists() {
        return Err(AppError::Conflict(
            "portable export destination already exists".into(),
        ));
    }
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let snapshot = unique_path(parent, "portable-snapshot.db");
    let archive_temp = unique_path(parent, "portable-archive.zip");
    let result = (|| {
        snapshot_database(database, &snapshot)?;
        // Secrets live in the OS credential vault, but even its lookup target is
        // stripped so the portable artifact carries no credential material.
        remove_credential_references(&snapshot)?;
        let database_bytes = fs::read(&snapshot)?;
        let settings_bytes = portable_settings(&snapshot)?;
        let connection = Connection::open(&snapshot)?;
        let manifest = PortableManifest {
            format_version: FORMAT_VERSION,
            app_version: env!("CARGO_PKG_VERSION").into(),
            schema_version: schema_version(&connection)?,
            created_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            database_sha256: sha256(&database_bytes),
            settings_sha256: sha256(&settings_bytes),
            encrypted: false,
        };
        let file = File::create(&archive_temp)?;
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer
            .start_file(MANIFEST_NAME, options)
            .map_err(zip_error)?;
        writer.write_all(
            &serde_json::to_vec_pretty(&manifest).map_err(|e| invalid(e.to_string()))?,
        )?;
        writer.start_file(DB_NAME, options).map_err(zip_error)?;
        writer.write_all(&database_bytes)?;
        writer
            .start_file(SETTINGS_NAME, options)
            .map_err(zip_error)?;
        writer.write_all(&settings_bytes)?;
        writer.finish().map_err(zip_error)?;
        fs::rename(&archive_temp, destination)?;
        Ok(manifest)
    })();
    let _ = fs::remove_file(snapshot);
    let _ = fs::remove_file(archive_temp);
    result
}

fn read_archive(path: &Path) -> Result<(PortableManifest, Vec<u8>), AppError> {
    if fs::metadata(path)?.len() > MAX_ARCHIVE_BYTES {
        return Err(invalid("portable archive is too large"));
    }
    let mut archive = ZipArchive::new(File::open(path)?).map_err(zip_error)?;
    if archive.len() != MAX_ENTRIES {
        return Err(invalid(
            "archive must contain exactly the allowlisted files",
        ));
    }
    let mut manifest_bytes = None;
    let mut database_bytes = None;
    let mut settings_bytes = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(zip_error)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| invalid("archive contains an unsafe path"))?;
        let name = enclosed.to_string_lossy().replace('\\', "/");
        if entry.is_dir() || entry.size() > MAX_ENTRY_BYTES {
            return Err(invalid("archive entry is invalid or too large"));
        }
        let mut bytes = Vec::new();
        entry.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_ENTRY_BYTES {
            return Err(invalid("archive entry expands beyond the size limit"));
        }
        match name.as_str() {
            MANIFEST_NAME if manifest_bytes.is_none() => manifest_bytes = Some(bytes),
            DB_NAME if database_bytes.is_none() => database_bytes = Some(bytes),
            SETTINGS_NAME if settings_bytes.is_none() => settings_bytes = Some(bytes),
            _ => {
                return Err(invalid(
                    "archive contains a duplicate or non-allowlisted file",
                ))
            }
        }
    }
    let manifest: PortableManifest =
        serde_json::from_slice(&manifest_bytes.ok_or_else(|| invalid("manifest is missing"))?)
            .map_err(|error| invalid(format!("manifest is invalid: {error}")))?;
    if manifest.format_version != FORMAT_VERSION || manifest.encrypted {
        return Err(invalid("unsupported portable archive format"));
    }
    if manifest.schema_version > supported_schema() {
        return Err(invalid("backup was created by a newer database schema"));
    }
    let database = database_bytes.ok_or_else(|| invalid("database is missing"))?;
    let settings = settings_bytes.ok_or_else(|| invalid("settings are missing"))?;
    if sha256(&database) != manifest.database_sha256
        || sha256(&settings) != manifest.settings_sha256
    {
        return Err(invalid("portable archive hash verification failed"));
    }
    let settings_value: serde_json::Value = serde_json::from_slice(&settings)
        .map_err(|error| invalid(format!("settings are invalid: {error}")))?;
    if settings_value.as_object().map(|v| v.len()) != Some(1)
        || !settings_value
            .get("providers")
            .is_some_and(|v| v.is_array())
    {
        return Err(invalid("settings contain non-allowlisted fields"));
    }
    for provider in settings_value["providers"]
        .as_array()
        .expect("checked above")
    {
        let object = provider
            .as_object()
            .ok_or_else(|| invalid("provider setting must be an object"))?;
        const ALLOWED: [&str; 4] = ["name", "baseUrl", "defaultModel", "enabled"];
        if object.keys().any(|key| !ALLOWED.contains(&key.as_str()))
            || !object.get("name").is_some_and(|v| v.is_string())
            || !object.get("baseUrl").is_some_and(|v| v.is_string())
            || !object.get("defaultModel").is_some_and(|v| v.is_string())
            || !object.get("enabled").is_some_and(|v| v.is_boolean())
        {
            return Err(invalid("provider settings contain non-allowlisted fields"));
        }
    }
    Ok((manifest, database))
}

/// `dry_run` reads and validates the archive and a disposable candidate only;
/// it never creates a safety backup or pending-restore file. A real import
/// creates both, while replacement of the live DB happens only at next startup.
pub fn import_portable(
    archive: &Path,
    current_database: &Path,
    dry_run: bool,
) -> Result<ImportReport, AppError> {
    let (manifest, database) = read_archive(archive)?;
    let parent = current_database.parent().unwrap_or_else(|| Path::new("."));
    let candidate = unique_path(parent, "portable-candidate.db");
    let mut candidate_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&candidate)?;
    candidate_file.write_all(&database)?;
    candidate_file.sync_all()?;
    drop(candidate_file);
    let validation = validate_database(&candidate);
    if let Err(error) = validation {
        let _ = fs::remove_file(candidate);
        return Err(error);
    }
    if schema_version(&Connection::open_with_flags(
        &candidate,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)? != manifest.schema_version
    {
        let _ = fs::remove_file(candidate);
        return Err(invalid("manifest and database schema versions differ"));
    }
    let counts = record_counts(&candidate)?;
    if dry_run {
        fs::remove_file(candidate)?;
        return Ok(ImportReport {
            manifest: manifest.clone(),
            record_counts: counts,
            app_version: manifest.app_version,
            schema_version: manifest.schema_version,
            dry_run: true,
            pending_restore: None,
            safety_backup: None,
        });
    }
    let backups = parent.join("backups");
    fs::create_dir_all(&backups)?;
    let safety = unique_path(&backups, "pre-portable-import.db");
    snapshot_database(current_database, &safety)?;
    let pending = current_database.with_extension("restore-pending.db");
    let pending_temp = current_database.with_extension("restore-pending.tmp");
    if pending.exists() || pending_temp.exists() {
        let _ = fs::remove_file(&candidate);
        return Err(AppError::Conflict(
            "a portable restore is already pending".into(),
        ));
    }
    fs::rename(&candidate, &pending_temp)?;
    fs::rename(&pending_temp, &pending)?;
    Ok(ImportReport {
        manifest: manifest.clone(),
        record_counts: counts,
        app_version: manifest.app_version,
        schema_version: manifest.schema_version,
        dry_run: false,
        pending_restore: Some(pending.to_string_lossy().into_owned()),
        safety_backup: Some(safety.to_string_lossy().into_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn database(path: &Path, marker: &str) {
        drop(super::super::db::Database::open_and_migrate(path).unwrap());
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch("CREATE TABLE content(value TEXT);")
            .unwrap();
        connection.execute("INSERT INTO provider_configs(name,base_url,default_model,credential_target,enabled) VALUES('openai','https://example.test','m','SECRET-TARGET',1)", []).unwrap();
        connection
            .execute("INSERT INTO content VALUES(?1)", [marker])
            .unwrap();
    }

    fn write_archive(path: &Path, manifest: &PortableManifest, db: &[u8], settings: &[u8]) {
        let mut writer = ZipWriter::new(File::create(path).unwrap());
        let options = SimpleFileOptions::default();
        writer.start_file(MANIFEST_NAME, options).unwrap();
        writer
            .write_all(&serde_json::to_vec(manifest).unwrap())
            .unwrap();
        writer.start_file(DB_NAME, options).unwrap();
        writer.write_all(db).unwrap();
        writer.start_file(SETTINGS_NAME, options).unwrap();
        writer.write_all(settings).unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn chinese_path_snapshot_is_cross_connection_consistent_and_staged() {
        let directory = tempdir().unwrap();
        let root = directory.path().join("中文 备份");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("职业.db");
        database(&source, "old");
        let second = Connection::open(&source).unwrap();
        second
            .execute("INSERT INTO content VALUES('committed elsewhere')", [])
            .unwrap();
        let archive = root.join("我的资料.ccbackup");
        let manifest = export_portable(&source, &archive, true).unwrap();
        assert_eq!(manifest.schema_version, supported_schema());
        let target = root.join("当前.db");
        database(&target, "current");
        let dry = import_portable(&archive, &target, true).unwrap();
        assert!(dry.dry_run);
        assert!(!target.with_extension("restore-pending.db").exists());
        let report = import_portable(&archive, &target, false).unwrap();
        assert!(Path::new(report.safety_backup.as_ref().unwrap()).exists());
        let pending = target.with_extension("restore-pending.db");
        let imported = Connection::open(pending).unwrap();
        let count: i64 = imported
            .query_row("SELECT COUNT(*) FROM content", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
        let credential_target: String = imported
            .query_row("SELECT credential_target FROM provider_configs", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(credential_target.is_empty());
    }

    #[test]
    fn acknowledgement_is_mandatory_and_archive_has_no_secret_or_logs() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("career.db");
        database(&database_path, "x");
        let archive_path = directory.path().join("export.zip");
        assert!(export_portable(&database_path, &archive_path, false).is_err());
        export_portable(&database_path, &archive_path, true).unwrap();
        let mut archive = ZipArchive::new(File::open(archive_path).unwrap()).unwrap();
        let names = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(names, vec![MANIFEST_NAME, DB_NAME, SETTINGS_NAME]);
        let mut settings = String::new();
        archive
            .by_name(SETTINGS_NAME)
            .unwrap()
            .read_to_string(&mut settings)
            .unwrap();
        assert!(!settings.contains("SECRET-TARGET"));
    }

    #[test]
    fn export_never_overwrites_existing_destination() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("career.db");
        database(&database_path, "x");
        let archive_path = directory.path().join("existing.ccbackup");
        fs::write(&archive_path, b"keep-me").unwrap();
        assert!(matches!(
            export_portable(&database_path, &archive_path, true),
            Err(AppError::Conflict(_))
        ));
        assert_eq!(fs::read(&archive_path).unwrap(), b"keep-me");
    }

    #[test]
    fn import_rejects_schema_migrations_only_database() {
        let directory = tempdir().unwrap();
        let current = directory.path().join("current.db");
        database(&current, "current");
        let fake = directory.path().join("fake.db");
        let connection = Connection::open(&fake).unwrap();
        connection.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at TEXT NOT NULL); INSERT INTO schema_migrations VALUES(1,'fake','now')").unwrap();
        drop(connection);
        let bytes = fs::read(&fake).unwrap();
        let settings = br#"{"providers":[]}"#;
        let manifest = PortableManifest {
            format_version: FORMAT_VERSION,
            app_version: "test".into(),
            schema_version: 1,
            created_unix_seconds: 1,
            database_sha256: sha256(&bytes),
            settings_sha256: sha256(settings),
            encrypted: false,
        };
        let archive = directory.path().join("fake.ccbackup");
        write_archive(&archive, &manifest, &bytes, settings);
        assert!(matches!(
            import_portable(&archive, &current, true),
            Err(AppError::IncompatibleSchema(_))
        ));
    }

    #[test]
    fn rejects_hash_corruption_future_schema_and_zip_slip() {
        let directory = tempdir().unwrap();
        let db = directory.path().join("career.db");
        database(&db, "x");
        let archive_path = directory.path().join("bad.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        writer.start_file("../manifest.json", options).unwrap();
        writer.write_all(b"{}").unwrap();
        writer.start_file(DB_NAME, options).unwrap();
        writer.write_all(b"bad").unwrap();
        writer.start_file(SETTINGS_NAME, options).unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap();
        assert!(import_portable(&archive_path, &db, true).is_err());

        fs::remove_file(&archive_path).unwrap();
        export_portable(&db, &archive_path, true).unwrap();
        let (mut manifest, database_bytes) = read_archive(&archive_path).unwrap();
        let settings = br#"{"providers":[]}"#;
        manifest.settings_sha256 = "0".repeat(64);
        write_archive(&archive_path, &manifest, &database_bytes, settings);
        assert!(import_portable(&archive_path, &db, true).is_err());

        manifest.settings_sha256 = sha256(settings);
        manifest.schema_version = supported_schema() + 1;
        write_archive(&archive_path, &manifest, &database_bytes, settings);
        assert!(import_portable(&archive_path, &db, true).is_err());

        manifest.schema_version = supported_schema();
        let corrupt_db = b"not sqlite";
        manifest.database_sha256 = sha256(corrupt_db);
        write_archive(&archive_path, &manifest, corrupt_db, settings);
        assert!(import_portable(&archive_path, &db, true).is_err());
    }

    #[test]
    fn rejects_zip_bomb_sized_expansion_before_extraction() {
        let directory = tempdir().unwrap();
        let db = directory.path().join("career.db");
        database(&db, "x");
        let archive_path = directory.path().join("bomb.zip");
        let mut writer = ZipWriter::new(File::create(&archive_path).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file(MANIFEST_NAME, options).unwrap();
        writer.write_all(b"{}").unwrap();
        writer.start_file(DB_NAME, options).unwrap();
        writer
            .write_all(&vec![0_u8; MAX_ENTRY_BYTES as usize + 1])
            .unwrap();
        writer.start_file(SETTINGS_NAME, options).unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap();
        assert!(import_portable(&archive_path, &db, true).is_err());
    }
}
