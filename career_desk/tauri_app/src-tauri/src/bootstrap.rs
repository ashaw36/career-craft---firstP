use crate::infra::db::Database;
use crate::interface::{commands::*, tasks::TaskRegistry, tauri as system_commands};
#[cfg(feature = "updater")]
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[tauri::command]
fn health() -> crate::error::Envelope<system_commands::HealthResponse> {
    system_commands::health()
}

#[tauri::command]
fn version() -> crate::error::Envelope<system_commands::VersionResponse> {
    system_commands::version()
}

fn task_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    format!(
        "task-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn run_operation(
    operation: &str,
    payload: Option<serde_json::Value>,
    cancel: &crate::infra::llm::CancellationToken,
) -> serde_json::Value {
    let envelope = match operation {
        "generate_resume" => generate_resume(payload),
        "chat_refine_resume" => {
            crate::interface::commands::resume_commands::chat_refine_resume_with_cancel(
                payload, cancel,
            )
        }
        "reframe_resume" => crate::interface::commands::reframe_resume_with_cancel(payload, cancel),
        "generate_learning_path" => get_learning_path(payload),
        "enrich_custom_skill_resources" => enrich_custom_skill_resources_with_cancel(payload, cancel),
        "export_resume_pdf" => export_resume_pdf(payload),
        "import_file" => import_file(payload),
        "import_experiences" => import_experiences(payload),
        "test_llm_connection" => test_llm_connection(payload),
        "parse_jd" => payload
            .and_then(|v| {
                v.get("jdText")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .map(parse_jd)
            .unwrap_or_else(|| {
                crate::error::Envelope::error(
                    crate::error::ErrorCode::Validation,
                    "jdText is required",
                )
            }),
        "structure_experience" => {
            crate::interface::commands::structure_experience_with_cancel(payload, cancel)
        }
        "recommend_persona_weights" => {
            crate::interface::commands::recommend_persona_weights_with_cancel(payload, cancel)
        }
        _ => crate::error::Envelope::unsupported(operation),
    };
    serde_json::to_value(envelope).unwrap_or_else(|_|serde_json::json!({"success":false,"error":{"code":"INTERNAL","message":"task result serialization failed"}}))
}

#[tauri::command]
fn start_background_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    payload: Option<serde_json::Value>,
) -> crate::error::Envelope<serde_json::Value> {
    let value = payload.unwrap_or_else(|| serde_json::json!({}));
    let Some(operation) = value
        .get("operation")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
    else {
        return crate::error::Envelope::error(
            crate::error::ErrorCode::Validation,
            "operation is required",
        );
    };
    let task_payload = value.get("payload").cloned();
    let id = task_id();
    let (event, cancel) = match state.tasks.register(id.clone(), operation.clone()) {
        Ok(value) => value,
        Err(message) => {
            return crate::error::Envelope::error(crate::error::ErrorCode::Unavailable, message)
        }
    };
    let _ = app.emit("careercraft://task", &event);
    let worker_id = id.clone();
    std::thread::spawn(move || {
        use crate::application::llm_orchestration::Cancellation;
        let stream_app = app.clone();
        let stream_id = worker_id.clone();
        crate::interface::commands::set_stream_hook(Some(std::sync::Arc::new(
            move |stream_event| {
                let state = stream_app.state::<AppState>();
                if let Some(event) = state.tasks.append_stream_event(&stream_id, stream_event) {
                    let _ = stream_app.emit("careercraft://task", &event);
                }
            },
        )));
        let value = run_operation(&operation, task_payload, &cancel);
        crate::interface::commands::set_stream_hook(None);
        let state = app.state::<AppState>();
        let current = state.tasks.get(&worker_id);
        let cancelled = cancel.is_cancelled()
            || current
                .as_ref()
                .is_some_and(|x| x.state == crate::interface::tasks::TaskState::Cancelled);
        let mut event = current.unwrap_or(crate::interface::tasks::TaskEvent {
            task_id: worker_id.clone(),
            operation: operation.clone(),
            state: crate::interface::tasks::TaskState::Started,
            progress: None,
            result: None,
            error: None,
            events: vec![],
        });
        if cancelled {
            event.state = crate::interface::tasks::TaskState::Cancelled;
            event.progress = None;
        } else if value.get("success").and_then(serde_json::Value::as_bool) == Some(true) {
            event.state = crate::interface::tasks::TaskState::Completed;
            event.progress = Some(100.0);
            event.result = value.get("data").cloned();
            event.error = None;
        } else {
            event.state = crate::interface::tasks::TaskState::Failed;
            event.progress = None;
            event.error = value
                .get("error")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok());
        }
        state.tasks.update(event.clone());
        let _ = app.emit("careercraft://task", &event);
    });
    crate::error::Envelope::ok(serde_json::json!({"taskId":id}))
}

#[tauri::command]
fn get_background_task(
    state: tauri::State<'_, AppState>,
    payload: Option<serde_json::Value>,
) -> crate::error::Envelope<serde_json::Value> {
    let id = payload
        .as_ref()
        .and_then(|v| v.get("taskId"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    match state.tasks.get(id) {
        Some(v) => crate::error::Envelope::ok(serde_json::to_value(v).unwrap_or_default()),
        None => crate::error::Envelope::error(crate::error::ErrorCode::NotFound, "task not found"),
    }
}
#[tauri::command]
fn cancel_background_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    payload: Option<serde_json::Value>,
) -> crate::error::Envelope<serde_json::Value> {
    let id = payload
        .as_ref()
        .and_then(|v| v.get("taskId"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    match state.tasks.cancel(id) {
        Some(v) => {
            let _ = app.emit("careercraft://task", &v);
            crate::error::Envelope::ok(serde_json::to_value(v).unwrap_or_default())
        }
        None => crate::error::Envelope::error(crate::error::ErrorCode::NotFound, "task not found"),
    }
}
#[tauri::command]
fn list_backups(_payload: Option<serde_json::Value>) -> crate::error::Envelope<serde_json::Value> {
    crate::infra::db::list_backups()
        .map(|v| serde_json::to_value(v).unwrap_or_default())
        .into()
}
#[tauri::command]
fn create_backup(_payload: Option<serde_json::Value>) -> crate::error::Envelope<serde_json::Value> {
    crate::infra::db::create_runtime_backup()
        .map(|v| serde_json::to_value(v).unwrap_or_default())
        .into()
}
#[tauri::command]
fn restore_backup(payload: Option<serde_json::Value>) -> crate::error::Envelope<serde_json::Value> {
    let value = payload.unwrap_or_default();
    let Some(name) = value.get("name").and_then(serde_json::Value::as_str) else {
        return crate::error::Envelope::error(
            crate::error::ErrorCode::Validation,
            "backup name is required",
        );
    };
    crate::infra::db::stage_restore_backup(name)
        .map(|_| serde_json::json!({"staged":true,"restartRequired":true}))
        .into()
}

pub struct AppState {
    pub database: Mutex<Database>,
    pub tasks: TaskRegistry,
    #[cfg(feature = "updater")]
    updater: Arc<Mutex<UpdaterRuntime>>,
}
#[cfg(feature = "updater")]
#[derive(Default)]
struct UpdaterRuntime {
    machine: crate::infra::updater::UpdateMachine,
    staged: Option<Vec<u8>>,
    version: Option<String>,
}

#[cfg(feature = "updater")]
fn update_journal(data_dir: &std::path::Path) -> crate::infra::update_recovery::JournalStore {
    crate::infra::update_recovery::JournalStore::new(
        data_dir.join("update-recovery").join("journal.json"),
    )
}

#[cfg(feature = "updater")]
fn stage_update_recovery(
    app: &tauri::AppHandle,
    target_version: &str,
    downloaded: &[u8],
) -> Result<(), crate::error::AppError> {
    use crate::infra::update_recovery::{DatabaseRecovery, SignedArtifact};
    let home = dirs::home_dir().ok_or_else(|| {
        crate::error::AppError::Unavailable("home directory is unavailable".into())
    })?;
    let data_dir = std::env::var_os("CAREERCRAFT_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".careercraft"));
    let recovery_dir = data_dir.join("update-recovery");
    std::fs::create_dir_all(&recovery_dir)?;
    let current_exe = std::env::current_exe()?;
    let retained = recovery_dir.join(format!("careercraft-{}.exe", env!("CARGO_PKG_VERSION")));
    std::fs::copy(&current_exe, &retained)?;
    let previous_max_schema_version = app
        .state::<AppState>()
        .database
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .connection()
        .query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
    let backup = crate::infra::db::create_runtime_backup()?;
    update_journal(&data_dir).stage_download(
        env!("CARGO_PKG_VERSION"),
        target_version,
        &crate::infra::update_recovery::sha256_bytes(downloaded),
        SignedArtifact {
            version: env!("CARGO_PKG_VERSION").into(),
            path: retained.clone(),
            sha256: crate::infra::update_recovery::sha256_file(&retained)?,
            // A copied running executable is not equivalent to a retained,
            // independently verified signed release artifact.
            signature_verified: false,
        },
        DatabaseRecovery {
            database_path: data_dir.join("career.db"),
            backup_path: std::path::PathBuf::from(backup.path),
            previous_max_schema_version,
        },
    )
}

fn startup_trace(message: &str) {
    let Some(path) = std::env::var_os("CAREERCRAFT_STARTUP_TRACE") else {
        return;
    };
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

fn visible_startup_error(message: &str, backup_dir: &std::path::Path) {
    let text = format!(
        "CareerCraft 无法启动。\n\n{message}\n\n备份目录：{}",
        backup_dir.display()
    );
    eprintln!("{}", crate::infra::security::redact_diagnostic(&text));
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let body = text
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let title = "CareerCraft 启动失败"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
#[cfg(feature = "updater")]
#[cfg(any())]
#[tauri::command]
async fn check_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> crate::error::Envelope<serde_json::Value> {
    use tauri_plugin_updater::UpdaterExt;
    {
        let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
        if runtime.machine.begin_check().is_err() {
            return crate::error::Envelope::error(
                crate::error::ErrorCode::Conflict,
                "update operation already active",
            );
        }
    }
    let updater = match app.updater() {
        Ok(v) => v,
        Err(_) => {
            *state.updater.lock().unwrap_or_else(|p| p.into_inner()) = UpdaterRuntime::default();
            return crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed updater is not configured",
            );
        }
    };
    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.to_string();
            let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
            if runtime.machine.available().is_err() {
                return crate::error::Envelope::error(
                    crate::error::ErrorCode::Conflict,
                    "invalid update state",
                );
            }
            runtime.version = Some(version.clone());
            crate::error::Envelope::ok(
                serde_json::json!({"available":true,"version":version,"body":update.body,"date":update.date.map(|v|v.to_string())}),
            )
        }
        Ok(None) => {
            *state.updater.lock().unwrap_or_else(|p| p.into_inner()) = UpdaterRuntime::default();
            crate::error::Envelope::ok(serde_json::json!({"available":false}))
        }
        Err(_) => {
            *state.updater.lock().unwrap_or_else(|p| p.into_inner()) = UpdaterRuntime::default();
            crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed update check failed",
            )
        }
    }
}
#[cfg(feature = "updater")]
#[cfg(any())]
#[tauri::command]
async fn download_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> crate::error::Envelope<serde_json::Value> {
    use tauri_plugin_updater::UpdaterExt;
    {
        let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
        if runtime.machine.begin_download().is_err() {
            return crate::error::Envelope::error(
                crate::error::ErrorCode::Conflict,
                "check for an update before download",
            );
        }
    }
    let update = match app.updater().and_then(|u| Ok(u)) {
        Ok(updater) => match updater.check().await {
            Ok(Some(v)) => v,
            _ => {
                *state.updater.lock().unwrap_or_else(|p| p.into_inner()) =
                    UpdaterRuntime::default();
                return crate::error::Envelope::error(
                    crate::error::ErrorCode::Unavailable,
                    "signed update is no longer available",
                );
            }
        },
        Err(_) => {
            return crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed updater is not configured",
            )
        }
    };
    match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => {
            let size = bytes.len();
            let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
            if runtime.machine.trusted_transport_staged().is_err() {
                return crate::error::Envelope::error(
                    crate::error::ErrorCode::Conflict,
                    "invalid update state",
                );
            }
            runtime.staged = Some(bytes);
            crate::error::Envelope::ok(
                serde_json::json!({"staged":true,"version":runtime.version,"bytes":size}),
            )
        }
        Err(_) => {
            *state.updater.lock().unwrap_or_else(|p| p.into_inner()) = UpdaterRuntime::default();
            crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed update download or verification failed",
            )
        }
    }
}
#[cfg(feature = "updater")]
#[cfg(any())]
#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> crate::error::Envelope<serde_json::Value> {
    use tauri_plugin_updater::UpdaterExt;
    let bytes = {
        let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
        if runtime
            .machine
            .begin_apply(env!("CARGO_PKG_VERSION"))
            .is_err()
        {
            return crate::error::Envelope::error(
                crate::error::ErrorCode::Conflict,
                "download and verify an update before install",
            );
        }
        match runtime.staged.take() {
            Some(v) => v,
            None => {
                return crate::error::Envelope::error(
                    crate::error::ErrorCode::Conflict,
                    "staged update missing",
                )
            }
        }
    };
    let update = match app.updater().and_then(|u| Ok(u)) {
        Ok(updater) => match updater.check().await {
            Ok(Some(v)) => v,
            _ => {
                let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
                let _ = runtime.machine.apply_failed_and_rollback();
                return crate::error::Envelope::error(
                    crate::error::ErrorCode::Unavailable,
                    "update changed before installation",
                );
            }
        },
        Err(_) => {
            return crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed updater is not configured",
            )
        }
    };
    match update.install(bytes) {
        Ok(()) => {
            let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
            let _ = runtime.machine.applied();
            crate::error::Envelope::ok(
                serde_json::json!({"installed":true,"relaunchRequired":true,"version":runtime.version}),
            )
        }
        Err(_) => {
            let mut runtime = state.updater.lock().unwrap_or_else(|p| p.into_inner());
            let previous = runtime
                .machine
                .apply_failed_and_rollback()
                .unwrap_or(env!("CARGO_PKG_VERSION"))
                .to_owned();
            crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                &format!("update installation failed; current version retained: {previous}"),
            )
        }
    }
}
#[cfg(feature = "updater")]
fn reset_updater(runtime: &Arc<Mutex<UpdaterRuntime>>) {
    *runtime.lock().unwrap_or_else(|p| p.into_inner()) = UpdaterRuntime::default()
}
#[cfg(feature = "updater")]
#[tauri::command]
async fn check_update(
    app: tauri::AppHandle,
) -> Result<crate::error::Envelope<serde_json::Value>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let runtime = app.state::<AppState>().updater.clone();
    {
        let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
        if state.machine.begin_check().is_err() {
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Conflict,
                "update operation already active",
            ));
        }
    }
    let updater = match app.updater() {
        Ok(v) => v,
        Err(_) => {
            reset_updater(&runtime);
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed updater is not configured",
            ));
        }
    };
    Ok(match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.to_string();
            let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
            if state.machine.available().is_err() {
                return Ok(crate::error::Envelope::error(
                    crate::error::ErrorCode::Conflict,
                    "invalid update state",
                ));
            }
            state.version = Some(version.clone());
            crate::error::Envelope::ok(
                serde_json::json!({"available":true,"version":version,"body":update.body,"date":update.date.map(|v|v.to_string())}),
            )
        }
        Ok(None) => {
            reset_updater(&runtime);
            crate::error::Envelope::ok(serde_json::json!({"available":false}))
        }
        Err(_) => {
            reset_updater(&runtime);
            crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed update check failed",
            )
        }
    })
}
#[cfg(feature = "updater")]
#[tauri::command]
async fn download_update(
    app: tauri::AppHandle,
) -> Result<crate::error::Envelope<serde_json::Value>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let runtime = app.state::<AppState>().updater.clone();
    {
        let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
        if state.machine.begin_download().is_err() {
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Conflict,
                "check for an update before download",
            ));
        }
    }
    let updater = match app.updater() {
        Ok(v) => v,
        Err(_) => {
            reset_updater(&runtime);
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed updater is not configured",
            ));
        }
    };
    let update = match updater.check().await {
        Ok(Some(v)) => v,
        _ => {
            reset_updater(&runtime);
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed update is no longer available",
            ));
        }
    };
    let target_version = update.version.to_string();
    Ok(match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => {
            let size = bytes.len();
            if stage_update_recovery(&app, &target_version, &bytes).is_err() {
                reset_updater(&runtime);
                return Ok(crate::error::Envelope::error(
                    crate::error::ErrorCode::Unavailable,
                    "update downloaded but recovery journal or database backup could not be staged",
                ));
            }
            let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
            if state.machine.trusted_transport_staged().is_err() {
                return Ok(crate::error::Envelope::error(
                    crate::error::ErrorCode::Conflict,
                    "invalid update state",
                ));
            }
            state.staged = Some(bytes);
            crate::error::Envelope::ok(
                serde_json::json!({"staged":true,"version":state.version,"bytes":size}),
            )
        }
        Err(_) => {
            reset_updater(&runtime);
            crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed update download or verification failed",
            )
        }
    })
}
#[cfg(feature = "updater")]
#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
) -> Result<crate::error::Envelope<serde_json::Value>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let runtime = app.state::<AppState>().updater.clone();
    let bytes = {
        let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
        if state
            .machine
            .begin_apply(env!("CARGO_PKG_VERSION"))
            .is_err()
        {
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Conflict,
                "download and verify an update before install",
            ));
        }
        match state.staged.take() {
            Some(v) => v,
            None => {
                return Ok(crate::error::Envelope::error(
                    crate::error::ErrorCode::Conflict,
                    "staged update missing",
                ))
            }
        }
    };
    let updater = match app.updater() {
        Ok(v) => v,
        Err(_) => {
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "signed updater is not configured",
            ))
        }
    };
    let update = match updater.check().await {
        Ok(Some(v)) => v,
        _ => {
            let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
            let _ = state.machine.apply_failed_and_rollback();
            return Ok(crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                "update changed before installation",
            ));
        }
    };
    let home = dirs::home_dir().ok_or_else(|| "home directory is unavailable".to_owned())?;
    let data_dir = std::env::var_os("CAREERCRAFT_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".careercraft"));
    let journal = update_journal(&data_dir);
    if journal
        .mark_pending_install()
        .and_then(|_| journal.mark_awaiting_health())
        .is_err()
    {
        return Ok(crate::error::Envelope::error(
            crate::error::ErrorCode::Unavailable,
            "persistent update recovery journal could not be armed",
        ));
    }
    Ok(match update.install(bytes) {
        Ok(()) => {
            let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
            let _ = state.machine.applied();
            crate::error::Envelope::ok(
                serde_json::json!({"installed":true,"relaunchRequired":true,"version":state.version}),
            )
        }
        Err(_) => {
            let _ = journal.mark_install_failed();
            let mut state = runtime.lock().unwrap_or_else(|p| p.into_inner());
            let previous = state
                .machine
                .apply_failed_and_rollback()
                .unwrap_or(env!("CARGO_PKG_VERSION"))
                .to_owned();
            crate::error::Envelope::error(
                crate::error::ErrorCode::Unavailable,
                format!("update installation failed; current version retained: {previous}"),
            )
        }
    })
}
#[cfg(not(feature = "updater"))]
#[tauri::command]
fn check_update() -> crate::error::Envelope<serde_json::Value> {
    crate::error::Envelope::error(
        crate::error::ErrorCode::Unavailable,
        "updater feature is disabled",
    )
}
#[cfg(not(feature = "updater"))]
#[tauri::command]
fn download_update() -> crate::error::Envelope<serde_json::Value> {
    crate::error::Envelope::error(
        crate::error::ErrorCode::Unavailable,
        "updater feature is disabled",
    )
}
#[cfg(not(feature = "updater"))]
#[tauri::command]
fn install_update() -> crate::error::Envelope<serde_json::Value> {
    crate::error::Envelope::error(
        crate::error::ErrorCode::Unavailable,
        "updater feature is disabled",
    )
}

pub fn run() {
    startup_trace("run:start");
    let Some(home) = dirs::home_dir() else {
        visible_startup_error("无法找到用户主目录。", std::path::Path::new("未知"));
        return;
    };
    startup_trace(&format!("home:{}", home.display()));
    // E2E runs must never touch a developer's real profile. The override is
    // inherited by the WDIO-managed child process and is otherwise inert.
    let data_dir = std::env::var_os("CAREERCRAFT_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".careercraft"));
    let db_path = data_dir.join("career.db");
    let backup_dir = data_dir.join("backups");
    if let Err(error) = crate::infra::db::apply_pending_restore(&db_path) {
        visible_startup_error(&format!("无法应用待恢复的备份：{error}"), &backup_dir);
        return;
    }
    startup_trace("restore:ok");
    #[cfg(feature = "updater")]
    let update_health_pending = match update_journal(&data_dir)
        .startup_decision(env!("CARGO_PKG_VERSION"))
    {
        Ok(crate::infra::update_recovery::StartupDecision::None) => false,
        Ok(crate::infra::update_recovery::StartupDecision::ProceedHealthCheck) => true,
        Ok(crate::infra::update_recovery::StartupDecision::Recover(plan)) => {
            let destination = match std::env::current_exe() {
                Ok(path) => path,
                Err(error) => {
                    visible_startup_error(
                        &format!("Update recovery cannot locate the installed executable: {error}"),
                        &backup_dir,
                    );
                    return;
                }
            };
            match update_journal(&data_dir).recover(
                &plan,
                &crate::infra::update_recovery::VerifiedFileInstaller,
                &destination,
            ) {
                Ok(()) => visible_startup_error(
                    "The previous verified version was restored. Restart CareerCraft.",
                    &backup_dir,
                ),
                Err(error) => visible_startup_error(
                    &format!(
                        "Update recovery is required but cannot proceed automatically: {error}. A retained, independently verified signed previous package is required."
                    ),
                    &backup_dir,
                ),
            }
            return;
        }
        Err(error) => {
            visible_startup_error(
                &format!("Update recovery journal validation failed: {error}"),
                &backup_dir,
            );
            return;
        }
    };
    let database = match Database::open_and_migrate(&db_path) {
        Ok(v) => v,
        Err(error) => {
            visible_startup_error(&format!("数据库校验或迁移失败：{error}"), &backup_dir);
            return;
        }
    };
    startup_trace("database:ok");
    if let Err(error) = crate::infra::db::configure_runtime_path(&db_path) {
        visible_startup_error(&format!("数据库运行时初始化失败：{error}"), &backup_dir);
        return;
    }
    startup_trace("runtime:ok");
    #[cfg(feature = "updater")]
    if update_health_pending {
        if let Err(error) = update_journal(&data_dir).commit_health(env!("CARGO_PKG_VERSION")) {
            visible_startup_error(
                &format!("Update startup health could not be committed: {error}"),
                &backup_dir,
            );
            return;
        }
        startup_trace("update-health:committed");
    }
    startup_trace("tauri:build");
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(feature = "wdio")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    #[cfg(feature = "updater")]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .manage(AppState {
            database: Mutex::new(database),
            tasks: TaskRegistry::default(),
            #[cfg(feature = "updater")]
            updater: Arc::new(Mutex::new(UpdaterRuntime::default())),
        })
        .invoke_handler(tauri::generate_handler![
            health,
            version,
            start_background_task,
            get_background_task,
            cancel_background_task,
            list_backups,
            create_backup,
            export_portable_backup,
            inspect_portable_backup,
            import_portable_backup,
            restore_backup,
            write_text_file,
            check_update,
            download_update,
            install_update,
            get_experiences,
            save_experience,
            delete_experience,
            get_personas,
            get_persona_by_id,
            create_persona,
            update_persona,
            delete_persona,
            get_experiences_with_fit_score,
            update_fit_score,
            generate_resume,
            preview_resume,
            export_resume_pdf,
            chat_refine_resume,
            list_resume_versions,
            diff_resume_versions,
            restore_resume_version,
            get_settings,
            save_settings,
            test_llm_connection,
            import_experiences,
            import_file,
            parse_jd,
            match_job,
            list_jobs,
            delete_job,
            get_job_matches,
            get_job_status_events,
            update_match_status,
            reframe_resume,
            get_reframe_results,
            update_reframe,
            reset_reframe,
            get_learning_path,
            get_learning_paths_by_source,
            delete_learning_path,
            get_skill_graph,
            get_skill_resources,
            search_skills,
            list_custom_skills,
            create_custom_skill,
            update_custom_skill,
            delete_custom_skill,
            simulate_skill_what_if,
            update_learning_progress,
            complete_learning_to_experience,
            reset_fit_score,
            delete_provider,
            open_external_url,
            collect_job_url
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CareerCraft");
}
