//! Versioned command facade. Commands perform work or return a specific domain/configuration error.
use crate::{
    application::{
        experiences, fit_scores, personas,
        ports::{ExperienceRepository, PersonaRepository, SecretStore},
    },
    domain::entities::*,
    error::{AppError, Envelope, ErrorCode},
    infra::{
        repositories::{
            append_experience_revision, SqliteExperienceRepository, SqlitePersonaRepository,
        },
        secrets::WindowsCredentialStore,
    },
};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
type StreamHook = Arc<dyn Fn(crate::domain::llm::StreamEvent)>;
type CustomSkillFields<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    String,
    String,
    u8,
    String,
);
#[cfg(test)]
type CommandFn = fn(Option<Value>) -> Envelope<Value>;
#[cfg(test)]
type CommandCase = (&'static str, CommandFn);
thread_local! {static STREAM_HOOK:RefCell<Option<StreamHook>> = RefCell::new(None)}
pub(crate) fn set_stream_hook(hook: Option<Arc<dyn Fn(crate::domain::llm::StreamEvent)>>) {
    STREAM_HOOK.with(|slot| *slot.borrow_mut() = hook)
}
fn emit_stream_event(event: crate::domain::llm::StreamEvent) {
    STREAM_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow().as_ref() {
            hook(event)
        }
    })
}

static OPEN_TOKENS: OnceLock<Mutex<HashMap<String, (String, Instant)>>> = OnceLock::new();
fn issue_open_token(url: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let token = format!(
        "open-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut tokens = OPEN_TOKENS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("open token registry poisoned");
    tokens.retain(|_, (_, at)| at.elapsed() < Duration::from_secs(120));
    tokens.insert(token.clone(), (url.into(), Instant::now()));
    token
}
fn consume_open_token(token: &str) -> Result<String, AppError> {
    let mut tokens = OPEN_TOKENS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::Internal)?;
    let (url, at) = tokens
        .remove(token)
        .ok_or_else(|| AppError::Validation("open token is invalid or already used".into()))?;
    if at.elapsed() > Duration::from_secs(120) {
        return Err(AppError::Validation("open token expired".into()));
    }
    Ok(url)
}

#[path = "resume_commands.rs"]
pub(crate) mod resume_commands;
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn generate_resume(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::generate_resume(payload)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn preview_resume(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::preview_resume(payload)
}
pub fn check_update(_payload: Option<Value>) -> Envelope<Value> {
    Envelope::error(
        ErrorCode::Unavailable,
        "updater command requires the desktop updater adapter",
    )
}
pub fn download_update(_payload: Option<Value>) -> Envelope<Value> {
    Envelope::error(
        ErrorCode::Unavailable,
        "updater command requires the desktop updater adapter",
    )
}
pub fn install_update(_payload: Option<Value>) -> Envelope<Value> {
    Envelope::error(
        ErrorCode::Unavailable,
        "updater command requires the desktop updater adapter",
    )
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn export_resume_pdf(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::export_resume_pdf(payload)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn chat_refine_resume(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::chat_refine_resume(payload)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn list_resume_versions(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::list_resume_versions(payload)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn diff_resume_versions(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::diff_resume_versions(payload)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn restore_resume_version(payload: Option<Value>) -> Envelope<Value> {
    resume_commands::restore_resume_version(payload)
}

pub const CONTRACT_COMMANDS: [&str; 33] = [
    "getExperiences",
    "saveExperience",
    "deleteExperience",
    "getPersonas",
    "getPersonaById",
    "createPersona",
    "updatePersona",
    "deletePersona",
    "getExperiencesWithFitScore",
    "updateFitScore",
    "generateResume",
    "exportResumePDF",
    "chatRefineResume",
    "getSettings",
    "saveSettings",
    "testLLMConnection",
    "importExperiences",
    "importFile",
    "parseJD",
    "matchJob",
    "listJobs",
    "deleteJob",
    "getJobMatches",
    "updateMatchStatus",
    "reframeResume",
    "getReframeResults",
    "updateReframe",
    "resetReframe",
    "getLearningPath",
    "getLearningPathsBySource",
    "getSkillGraph",
    "getSkillResources",
    "searchSkills",
];

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    let mut h = DefaultHasher::new();
    parts.hash(&mut h);
    format!("{prefix}-{:016x}", h.finish())
}

fn connection() -> Result<Connection, AppError> {
    crate::infra::db::open_runtime_connection()
}
fn runtime_database_file() -> Result<std::path::PathBuf, AppError> {
    let connection = connection()?;
    let path: String = connection.query_row(
        "SELECT file FROM pragma_database_list WHERE name='main'",
        [],
        |row| row.get(0),
    )?;
    Ok(path.into())
}
fn portable_path(value: &str, must_exist: bool) -> Result<std::path::PathBuf, AppError> {
    let path = std::path::PathBuf::from(value);
    let allowed = path
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "zip" | "ccbackup"));
    if !path.is_absolute() || !allowed {
        return Err(AppError::Validation(
            "portable backup path must be absolute and end in .ccbackup or .zip".into(),
        ));
    }
    if must_exist {
        let metadata = std::fs::metadata(&path)?;
        if !metadata.is_file() || metadata.len() > 300 * 1024 * 1024 {
            return Err(AppError::Validation(
                "portable backup must be a regular file no larger than 300 MiB".into(),
            ));
        }
    }
    Ok(path)
}

fn export_text_path(value: &str) -> Result<std::path::PathBuf, AppError> {
    let path = std::path::PathBuf::from(value);
    let allowed = path
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "md" | "markdown" | "txt"));
    if !path.is_absolute() || !allowed {
        return Err(AppError::Validation(
            "export path must be absolute and end in .md / .markdown / .txt".into(),
        ));
    }
    Ok(path)
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn write_text_file(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let value = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let path = export_text_path(required(&value, "destinationPath")?)?;
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::Validation("content is required".into()))?;
        if content.len() > 8 * 1024 * 1024 {
            return Err(AppError::Validation(
                "text content must be no larger than 8 MiB".into(),
            ));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(AppError::from)?;
        }
        std::fs::write(&path, content.as_bytes()).map_err(AppError::from)?;
        Ok(json!({"saved":true,"path":path.to_string_lossy()}))
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn export_portable_backup(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let value = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let destination = portable_path(required(&value, "destinationPath")?, false)?;
        let acknowledged =
            value.get("acknowledgeUnencrypted").and_then(Value::as_bool) == Some(true);
        let manifest = crate::infra::portable_backup::export_portable(
            &runtime_database_file()?,
            &destination,
            acknowledged,
        )?;
        Ok(
            json!({"manifest":manifest,"warning":"This backup is not encrypted and can be read by anyone who obtains it."}),
        )
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn inspect_portable_backup(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let value = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let report = crate::infra::portable_backup::import_portable(
            &portable_path(required(&value, "archivePath")?, true)?,
            &runtime_database_file()?,
            true,
        )?;
        serde_json::to_value(report).map_err(|_| AppError::Internal)
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn import_portable_backup(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let value = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        if value.get("confirmed").and_then(Value::as_bool) != Some(true) {
            return Err(AppError::Validation("confirmed must be true".into()));
        }
        let report = crate::infra::portable_backup::import_portable(
            &portable_path(required(&value, "archivePath")?, true)?,
            &runtime_database_file()?,
            false,
        )?;
        let mut output = serde_json::to_value(report).map_err(|_| AppError::Internal)?;
        output["restartRequired"] = Value::Bool(true);
        Ok(output)
    })())
}
fn reconcile_credential_target<S: SecretStore>(
    store: &S,
    old: Option<&str>,
    new: &str,
    key: Option<&str>,
) -> Result<(), AppError> {
    if old.is_some_and(|v| v != new)
        && key.is_none()
        && store.exists(old.unwrap()).map_err(AppError::from)?
    {
        return Err(AppError::Validation(
            "credentialTarget changed; apiKey must be entered again".into(),
        ));
    }
    if let Some(key) = key {
        store.put(new, key).map_err(AppError::from)?;
        if let Some(old) = old.filter(|v| *v != new) {
            store.delete(old).map_err(AppError::from)?;
        }
    }
    Ok(())
}
fn credential_target(provider: &str, endpoint: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in endpoint.trim_end_matches('/').to_ascii_lowercase().bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!(
        "provider/{}/endpoint/{hash:016x}",
        provider.trim().to_ascii_lowercase()
    )
}
fn llm_generate(c: &Connection, prompt: String) -> Result<String, AppError> {
    use crate::{
        application::llm_orchestration::{
            generate_with_fallback, EventSink, RetryPolicy, RetrySleeper,
        },
        domain::llm::{GenerationRequest, LlmMessage, LlmRole, ModelRef, StreamEvent},
        infra::llm::{CancellationToken, OpenAiCompatibleProvider, ProviderConfig},
    };
    let mut s=c.prepare("SELECT name,base_url,default_model,credential_target FROM provider_configs WHERE enabled=1 ORDER BY name")?;
    let configs = s
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if configs.is_empty() {
        return Err(AppError::Validation(
            "no enabled LLM provider configured".into(),
        ));
    }
    let store = WindowsCredentialStore;
    let mut providers = Vec::new();
    let mut routes = Vec::new();
    for (name, url, model, target) in configs {
        let key = store.get(&target).map_err(AppError::from)?;
        providers.push(
            OpenAiCompatibleProvider::new(
                ProviderConfig {
                    provider: name.clone(),
                    base_url: url,
                    default_model: model.clone(),
                    credential_target: target,
                    enabled: true,
                },
                key,
            )
            .map_err(|e| AppError::Validation(e.message))?,
        );
        routes.push(ModelRef {
            provider: name,
            model,
        });
    }
    let refs = providers
        .iter()
        .map(|p| p as &dyn crate::application::llm_orchestration::LlmProvider)
        .collect::<Vec<_>>();
    struct Sleep;
    impl RetrySleeper for Sleep {
        fn sleep_ms(&self, n: u64) {
            std::thread::sleep(std::time::Duration::from_millis(n))
        }
    }
    struct Sink;
    impl EventSink for Sink {
        fn emit(&mut self, event: StreamEvent) {
            emit_stream_event(event)
        }
    }
    let request = GenerationRequest {
        messages: vec![LlmMessage {
            role: LlmRole::User,
            content: prompt,
        }],
        preferred: None,
        temperature: 0.2,
        max_output_tokens: 1200,
    };
    generate_with_fallback(
        &refs,
        &routes,
        &request,
        &RetryPolicy::default(),
        &Sleep,
        &CancellationToken::default(),
        &mut Sink,
    )
    .map(|r| r.text)
    .map_err(|e| AppError::Unavailable(e.message))
}
pub(crate) fn generate_for_persona(
    persona_id: Option<&str>,
    prompt: String,
    cancel: &crate::infra::llm::CancellationToken,
) -> Result<String, AppError> {
    generate_result_for_persona(persona_id, prompt, cancel).map(|result| result.text)
}
pub(crate) fn generate_for_persona_with_tokens(
    persona_id: Option<&str>,
    prompt: String,
    max_output_tokens: u32,
    cancel: &crate::infra::llm::CancellationToken,
) -> Result<String, AppError> {
    generate_request_for_persona(
        persona_id,
        crate::domain::llm::GenerationRequest {
            messages: vec![crate::domain::llm::LlmMessage {
                role: crate::domain::llm::LlmRole::User,
                content: prompt,
            }],
            preferred: None,
            temperature: 0.2,
            max_output_tokens,
        },
        cancel,
    )
    .map(|result| result.text)
}
pub(crate) fn generate_result_for_persona(
    persona_id: Option<&str>,
    prompt: String,
    cancel: &crate::infra::llm::CancellationToken,
) -> Result<crate::domain::llm::GenerationResult, AppError> {
    generate_request_for_persona(
        persona_id,
        crate::domain::llm::GenerationRequest {
            messages: vec![crate::domain::llm::LlmMessage {
                role: crate::domain::llm::LlmRole::User,
                content: prompt,
            }],
            preferred: None,
            temperature: 0.2,
            max_output_tokens: 1200,
        },
        cancel,
    )
}
fn generate_request_for_persona(
    persona_id: Option<&str>,
    request: crate::domain::llm::GenerationRequest,
    cancel: &crate::infra::llm::CancellationToken,
) -> Result<crate::domain::llm::GenerationResult, AppError> {
    use crate::{
        application::llm_orchestration::{
            generate_with_fallback, EventSink, RetryPolicy, RetrySleeper,
        },
        domain::llm::{ModelRef, StreamEvent},
        infra::llm::{OpenAiCompatibleProvider, ProviderConfig},
    };
    let c = connection()?;
    let preferred = persona_id.and_then(|id| {
        c.query_row(
            "SELECT preferred_model FROM personas WHERE id=?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
    });
    let mut s=c.prepare("SELECT name,base_url,default_model,credential_target FROM provider_configs WHERE enabled=1 ORDER BY name")?;
    let mut configs = s
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if configs.is_empty() {
        return Err(AppError::Validation(
            "no enabled LLM provider configured".into(),
        ));
    }
    if let Some(pref) = &preferred {
        configs.sort_by_key(|(name, _, model, _)| {
            if pref == model || pref == &format!("{name}/{model}") || pref == name {
                0
            } else {
                1
            }
        });
    }
    let store = WindowsCredentialStore;
    let mut providers = Vec::new();
    let mut routes = Vec::new();
    for (name, url, model, target) in configs {
        let key = store.get(&target).map_err(AppError::from)?;
        providers.push(
            OpenAiCompatibleProvider::new(
                ProviderConfig {
                    provider: name.clone(),
                    base_url: url,
                    default_model: model.clone(),
                    credential_target: target,
                    enabled: true,
                },
                key,
            )
            .map_err(|e| AppError::Validation(e.message))?,
        );
        routes.push(ModelRef {
            provider: name,
            model,
        });
    }
    let refs = providers
        .iter()
        .map(|p| p as &dyn crate::application::llm_orchestration::LlmProvider)
        .collect::<Vec<_>>();
    struct Sleep;
    impl RetrySleeper for Sleep {
        fn sleep_ms(&self, n: u64) {
            std::thread::sleep(std::time::Duration::from_millis(n))
        }
    }
    struct Sink;
    impl EventSink for Sink {
        fn emit(&mut self, event: StreamEvent) {
            emit_stream_event(event)
        }
    }
    generate_with_fallback(
        &refs,
        &routes,
        &request,
        &RetryPolicy::default(),
        &Sleep,
        cancel,
        &mut Sink,
    )
    .map_err(|e| match e.kind {
        crate::domain::llm::LlmErrorKind::Cancelled => AppError::Cancelled,
        crate::domain::llm::LlmErrorKind::Timeout => {
            AppError::Unavailable(format!("TIMEOUT: {}", e.message))
        }
        crate::domain::llm::LlmErrorKind::RateLimited => {
            AppError::Unavailable(format!("RATE_LIMITED: {}", e.message))
        }
        _ => AppError::Unavailable(e.message),
    })
}
fn ordered_model_routes(
    c: &Connection,
    persona_id: Option<&str>,
) -> Result<Vec<(String, String)>, AppError> {
    let preferred = persona_id.and_then(|id| {
        c.query_row(
            "SELECT preferred_model FROM personas WHERE id=?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
    });
    let mut s =
        c.prepare("SELECT name,default_model FROM provider_configs WHERE enabled=1 ORDER BY name")?;
    let mut routes = s
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(pref) = preferred {
        routes.sort_by_key(|(provider, model)| {
            if pref == *model || pref == *provider || pref == format!("{provider}/{model}") {
                0
            } else {
                1
            }
        });
    }
    Ok(routes)
}
fn first_route_cache(
    routes: &[(String, String)],
    request: &crate::domain::llm::GenerationRequest,
    operation: &str,
    prompt_version: &str,
    now: i64,
) -> Result<Option<crate::domain::llm::GenerationResult>, AppError> {
    if let Some((provider, model)) = routes.first() {
        let key = crate::infra::llm_cache::key(operation, prompt_version, provider, model, request);
        if let Some(hit) = crate::infra::llm_cache::get(&key, now)? {
            if hit.provider == *provider && hit.model == *model {
                return Ok(Some(hit));
            }
        }
    }
    Ok(None)
}
fn generate_request_for_persona_cached(
    persona_id: Option<&str>,
    request: crate::domain::llm::GenerationRequest,
    cancel: &crate::infra::llm::CancellationToken,
    operation: &str,
    prompt_version: &str,
) -> Result<(crate::domain::llm::GenerationResult, bool), AppError> {
    let _flight = crate::infra::llm_cache::single_flight();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let c = connection()?;
    let routes = ordered_model_routes(&c, persona_id)?;
    drop(c);
    // A cached fallback must never bypass a healthy preferred route. Therefore only
    // the first effective route is eligible before live fallback orchestration.
    if let Some(hit) = first_route_cache(&routes, &request, operation, prompt_version, now)? {
        return Ok((hit, true));
    }
    let result = generate_request_for_persona(persona_id, request.clone(), cancel)?;
    let key = crate::infra::llm_cache::key(
        operation,
        prompt_version,
        &result.provider,
        &result.model,
        &request,
    );
    crate::infra::llm_cache::put_success(&key, operation, prompt_version, &result, now)?;
    Ok((result, false))
}
fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str, AppError> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| AppError::Validation(format!("{key} is required")))
}
fn result<T: serde::Serialize>(v: Result<T, AppError>) -> Envelope<Value> {
    match v {
        Ok(x) => Envelope::ok(serde_json::to_value(x).unwrap_or(Value::Null)),
        Err(e) => Envelope::from(Err::<Value, _>(e)),
    }
}
pub(crate) fn structure_experience_with_cancel(
    payload: Option<Value>,
    cancel: &crate::infra::llm::CancellationToken,
) -> Envelope<Value> {
    structure_experience_with_generator(payload, cancel, |persona, request, cancel| {
        generate_request_for_persona_cached(
            persona,
            request.clone(),
            cancel,
            "structure_experience",
            crate::application::experience_structuring::PROMPT_VERSION,
        )
        .map_err(|e| crate::domain::llm::LlmError {
            kind: match e {
                AppError::Cancelled => crate::domain::llm::LlmErrorKind::Cancelled,
                _ => crate::domain::llm::LlmErrorKind::Provider,
            },
            message: format!("{e:?}"),
        })
    })
}
fn structure_experience_with_generator<F>(
    payload: Option<Value>,
    cancel: &crate::infra::llm::CancellationToken,
    mut generate: F,
) -> Envelope<Value>
where
    F: FnMut(
        Option<&str>,
        &crate::domain::llm::GenerationRequest,
        &crate::infra::llm::CancellationToken,
    )
        -> Result<(crate::domain::llm::GenerationResult, bool), crate::domain::llm::LlmError>,
{
    result((|| {
        let value = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let raw = required(&value, "rawDescription")?;
        let persona = value.get("personaId").and_then(Value::as_str);
        let mut cache_hit = false;
        let mut preview =
            crate::application::experience_structuring::structure_with_generator(raw, |request| {
                let (result, hit) = generate(persona, request, cancel)?;
                cache_hit = hit;
                Ok(result)
            })
            .map_err(|e| match e {
                crate::application::experience_structuring::StructureError::InvalidInput(m)
                | crate::application::experience_structuring::StructureError::InvalidOutput(m) => {
                    AppError::Validation(m)
                }
                crate::application::experience_structuring::StructureError::Llm(e) => {
                    match e.kind {
                        crate::domain::llm::LlmErrorKind::Cancelled => AppError::Cancelled,
                        _ => AppError::Unavailable(e.message),
                    }
                }
            })?;
        preview.cache_hit = cache_hit;
        Ok(preview)
    })())
}
fn exp_json(e: &Experience) -> Value {
    json!({"id":e.id,"userId":e.user_id,"type":match e.kind{ExperienceType::Work=>"work",ExperienceType::Project=>"project",ExperienceType::Education=>"education",ExperienceType::Certification=>"certification"},"title":e.title,"organization":e.organization,"startDate":e.start_date,"endDate":e.end_date,"rawDescription":e.raw_description,"structuredAchievements":e.structured_achievements,"skillsDemonstrated":e.skills_demonstrated,"industryTags":e.industry_tags,"educationLevel":e.education_level.as_ref().map(EducationLevel::as_str).unwrap_or("none"),"status":match e.status{ExperienceStatus::Draft=>"draft",ExperienceStatus::Confirmed=>"confirmed",ExperienceStatus::Discarded=>"discarded",ExperienceStatus::Archived=>"archived"},"version":e.version})
}
fn exp_json_with_overlaps(e: &Experience, values: &[Experience]) -> Value {
    let mut value = exp_json(e);
    let ids = experiences::overlapping_ids(e, values);
    value["overlapExperienceIds"] = json!(ids);
    value["warnings"] = if ids.is_empty() {
        json!([])
    } else {
        json!([{"code":"DATE_OVERLAP","experienceIds":ids}])
    };
    value
}
fn persona_json(p: Persona) -> Value {
    json!({"id":p.id,"userId":p.user_id,"name":p.name,"isDefault":p.is_default,"identityStatement":p.identity_statement,"careerNarrative":p.career_narrative,"toneStyle":p.tone_style,"capabilityWeights":p.capability_weights.into_iter().collect::<std::collections::BTreeMap<_,_>>(),"targetJobProfiles":p.target_job_profiles,"maxExperiences":p.max_experiences,"preferredModel":p.preferred_model})
}
fn exp_type(v: &str) -> Result<ExperienceType, AppError> {
    match v {
        "work" => Ok(ExperienceType::Work),
        "project" => Ok(ExperienceType::Project),
        "education" => Ok(ExperienceType::Education),
        "certification" => Ok(ExperienceType::Certification),
        _ => Err(AppError::Validation("invalid experience type".into())),
    }
}
fn exp_status(v: &str) -> Result<ExperienceStatus, AppError> {
    match v {
        "draft" => Ok(ExperienceStatus::Draft),
        "confirmed" => Ok(ExperienceStatus::Confirmed),
        "discarded" => Ok(ExperienceStatus::Discarded),
        "archived" => Ok(ExperienceStatus::Archived),
        _ => Err(AppError::Validation("invalid experience status".into())),
    }
}
fn education_level(value: Option<&Value>) -> Result<Option<EducationLevel>, AppError> {
    let Some(raw) = value.and_then(Value::as_str) else {
        return Ok(None);
    };
    if raw.eq_ignore_ascii_case("none") || raw.trim().is_empty() {
        return Ok(None);
    }
    EducationLevel::parse(raw)
        .ok_or_else(|| AppError::Validation("invalid educationLevel".into()))
        .map(Some)
}
fn str_vec(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_experiences(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let c = connection()?;
        let r = SqliteExperienceRepository::new(&c);
        let values = r.list(
            payload
                .as_ref()
                .and_then(|v| v.get("userId"))
                .and_then(Value::as_str)
                .unwrap_or("default"),
        )?;
        Ok(Value::Array(
            values
                .iter()
                .map(|e| exp_json_with_overlaps(e, &values))
                .collect(),
        ))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn save_experience(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        let r = SqliteExperienceRepository::new(&c);
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            let patch = ExperiencePatch {
                title: v.get("title").and_then(Value::as_str).map(str::to_owned),
                organization: v.get("organization").map(|x| x.as_str().map(str::to_owned)),
                start_date: v.get("startDate").map(|x| x.as_str().map(str::to_owned)),
                end_date: v.get("endDate").map(|x| x.as_str().map(str::to_owned)),
                status: v
                    .get("status")
                    .and_then(Value::as_str)
                    .map(exp_status)
                    .transpose()?,
                industry_tags: v.get("industryTags").map(|v| str_vec(Some(v))),
                education_level: if v.get("educationLevel").is_some() {
                    Some(education_level(v.get("educationLevel"))?)
                } else {
                    None
                },
                raw_description: v
                    .get("rawDescription")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                kind: v
                    .get("type")
                    .and_then(Value::as_str)
                    .map(exp_type)
                    .transpose()?,
                structured_achievements: v.get("structuredAchievements").map(|v| str_vec(Some(v))),
                skills_demonstrated: v.get("skillsDemonstrated").map(|v| str_vec(Some(v))),
            };
            let e = experiences::update(
                &r,
                id,
                v.get("version")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| AppError::Validation("version is required".into()))?
                    as u32,
                &patch,
            )
            .map_err(AppError::from)?;
            let values = r.list(&e.user_id)?;
            Ok(exp_json_with_overlaps(&e, &values))
        } else {
            let e = Experience {
                id: required(&v, "newId")?.into(),
                user_id: v
                    .get("userId")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .into(),
                kind: exp_type(required(&v, "type")?)?,
                title: required(&v, "title")?.into(),
                organization: v
                    .get("organization")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                start_date: v
                    .get("startDate")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                end_date: v.get("endDate").and_then(Value::as_str).map(str::to_owned),
                raw_description: v
                    .get("rawDescription")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                structured_achievements: str_vec(v.get("structuredAchievements")),
                skills_demonstrated: str_vec(v.get("skillsDemonstrated")),
                industry_tags: str_vec(v.get("industryTags")),
                education_level: education_level(v.get("educationLevel"))?,
                status: v
                    .get("status")
                    .and_then(Value::as_str)
                    .map(exp_status)
                    .transpose()?
                    .unwrap_or(ExperienceStatus::Draft),
                version: 1,
            };
            experiences::create(&r, &e).map_err(AppError::from)?;
            let values = r.list(&e.user_id)?;
            Ok(exp_json_with_overlaps(&e, &values))
        }
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn delete_experience(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let id = required(&v, "experienceId")?;
        let c = connection()?;
        let r = SqliteExperienceRepository::new(&c);
        let version = v
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| AppError::Validation("version is required".into()))?
            as u32;
        r.delete(id, version).map_err(AppError::from)?;
        Ok(json!({"deleted":true}))
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_personas(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let c = connection()?;
        let r = SqlitePersonaRepository::new(&c);
        Ok(Value::Array(
            r.list(
                payload
                    .as_ref()
                    .and_then(|v| v.get("userId"))
                    .and_then(Value::as_str)
                    .unwrap_or("default"),
            )?
            .into_iter()
            .map(persona_json)
            .collect(),
        ))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_persona_by_id(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        let r = SqlitePersonaRepository::new(&c);
        r.get(required(&v, "personaId")?)?
            .map(persona_json)
            .ok_or_else(|| AppError::NotFound("persona".into()))
    })())
}
fn persona_from(v: &Value) -> Result<Persona, AppError> {
    let weights = v
        .get("capabilityWeights")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_f64().map(|n| (k.clone(), n)))
                .collect()
        })
        .unwrap_or_default();
    Ok(Persona {
        id: required(v, "id")?.into(),
        user_id: v
            .get("userId")
            .and_then(Value::as_str)
            .unwrap_or("default")
            .into(),
        name: required(v, "name")?.into(),
        is_default: v.get("isDefault").and_then(Value::as_bool).unwrap_or(false),
        identity_statement: v
            .get("identityStatement")
            .and_then(Value::as_str)
            .map(str::to_owned),
        career_narrative: v
            .get("careerNarrative")
            .and_then(Value::as_str)
            .map(str::to_owned),
        tone_style: v
            .get("toneStyle")
            .and_then(Value::as_str)
            .map(str::to_owned),
        capability_weights: weights,
        target_job_profiles: str_vec(v.get("targetJobProfiles")),
        max_experiences: v.get("maxExperiences").and_then(Value::as_u64).unwrap_or(5) as u32,
        preferred_model: v
            .get("preferredModel")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn create_persona(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let p = persona_from(
            &payload.ok_or_else(|| AppError::Validation("payload is required".into()))?,
        )?;
        let c = connection()?;
        let r = SqlitePersonaRepository::new(&c);
        personas::create(&r, &p).map_err(AppError::from)?;
        Ok(persona_json(p))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_persona(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let id = required(&v, "personaId")?;
        let data = v.get("data").unwrap_or(&v);
        let weights = data
            .get("capabilityWeights")
            .and_then(Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| v.as_f64().map(|n| (k.clone(), n)))
                    .collect()
            });
        let p = PersonaPatch {
            name: data.get("name").and_then(Value::as_str).map(str::to_owned),
            identity_statement: data
                .get("identityStatement")
                .map(|x| x.as_str().map(str::to_owned)),
            career_narrative: data
                .get("careerNarrative")
                .map(|x| x.as_str().map(str::to_owned)),
            tone_style: data.get("toneStyle").map(|x| x.as_str().map(str::to_owned)),
            capability_weights: weights,
            target_job_profiles: data.get("targetJobProfiles").map(|x| str_vec(Some(x))),
            max_experiences: data
                .get("maxExperiences")
                .and_then(Value::as_u64)
                .map(|x| x as u32),
            preferred_model: data
                .get("preferredModel")
                .map(|x| x.as_str().map(str::to_owned)),
        };
        let c = connection()?;
        let r = SqlitePersonaRepository::new(&c);
        Ok(persona_json(
            personas::update(&r, id, &p).map_err(AppError::from)?,
        ))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn delete_persona(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        SqlitePersonaRepository::new(&c)
            .delete(required(&v, "personaId")?)
            .map_err(AppError::from)?;
        Ok(json!({"deleted":true}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_experiences_with_fit_score(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let id = required(&v, "personaId")?;
        let c = connection()?;
        let er = SqliteExperienceRepository::new(&c);
        let pr = SqlitePersonaRepository::new(&c);
        let rows = fit_scores::recalculate(&er, &pr, id).map_err(AppError::from)?;
        Ok(json!(rows.into_iter().map(|w|json!({"experienceId":w.experience_id,"relevanceScore":w.relevance_score*100.0,"userOverridden":w.user_overridden})).collect::<Vec<_>>()))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_fit_score(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        let pr = SqlitePersonaRepository::new(&c);
        let score = v
            .get("score")
            .and_then(Value::as_f64)
            .ok_or_else(|| AppError::Validation("score is required".into()))?
            / 100.0;
        let w = fit_scores::override_score(
            &pr,
            required(&v, "personaId")?,
            required(&v, "experienceId")?,
            score,
        )
        .map_err(AppError::from)?;
        Ok(
            json!({"experienceId":w.experience_id,"relevanceScore":w.relevance_score*100.0,"userOverridden":true}),
        )
    })())
}

pub fn recommend_persona_weights(payload: Option<Value>) -> Envelope<Value> {
    recommend_persona_weights_with_cancel(payload, &crate::infra::llm::CancellationToken::default())
}

pub(crate) fn recommend_persona_weights_with_cancel(
    payload: Option<Value>,
    cancel: &crate::infra::llm::CancellationToken,
) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let persona_id = required(&v, "personaId")?;
        let c = connection()?;
        let pr = SqlitePersonaRepository::new(&c);
        let er = SqliteExperienceRepository::new(&c);
        let persona = pr
            .get(persona_id)
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::NotFound("persona".into()))?;
        let experiences = er
            .list_confirmed(&persona.user_id)
            .map_err(AppError::from)?;
        if experiences.is_empty() {
            return Ok(json!({"personaId":persona_id,"scores":[],"source":"empty"}));
        }
        let catalog = experiences
            .iter()
            .map(|exp| {
                json!({
                    "experienceId": exp.id,
                    "title": exp.title,
                    "organization": exp.organization,
                    "type": format!("{:?}", exp.kind).to_lowercase(),
                    "skills": exp.skills_demonstrated,
                    "achievements": exp.structured_achievements,
                })
            })
            .collect::<Vec<_>>();
        let positioning = persona
            .identity_statement
            .clone()
            .unwrap_or_default();
        let targets = persona.target_job_profiles.join("、");
        let prompt = format!(
            "You score how well each experience supports a career persona.\nReturn ONE JSON object only: {{\"scores\":[{{\"experienceId\":\"...\",\"score\":0-100}}]}}.\nRules: include every experienceId exactly once; score is integer 0-100; higher means more relevant to the positioning; do not invent experiences; no markdown fences.\nPersona name: {}\nTarget roles: {}\nPositioning statement:\n{}\n\nExperiences JSON:\n{}",
            persona.name,
            targets,
            positioning,
            serde_json::to_string(&catalog).unwrap_or_else(|_| "[]".into())
        );
        let raw = generate_for_persona_with_tokens(Some(persona_id), prompt, 2048, cancel)?;
        let parsed: Value = serde_json::from_str(strip_llm_json_fence(&raw)).map_err(|_| {
            AppError::Unavailable("LLM weight recommendation must be valid JSON".into())
        })?;
        let rows = parsed
            .get("scores")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AppError::Unavailable("LLM weight recommendation missing scores".into())
            })?;
        let mut by_id = std::collections::BTreeMap::<String, f64>::new();
        for row in rows {
            let id = row
                .get("experienceId")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if id.is_empty() || !experiences.iter().any(|e| e.id == id) {
                continue;
            }
            let score = row
                .get("score")
                .and_then(Value::as_f64)
                .or_else(|| {
                    row.get("score")
                        .and_then(Value::as_i64)
                        .map(|n| n as f64)
                })
                .unwrap_or(50.0)
                .clamp(0.0, 100.0);
            by_id.insert(id.to_owned(), score);
        }
        let scores = experiences
            .iter()
            .map(|exp| {
                json!({
                    "experienceId": exp.id,
                    "relevanceScore": by_id.get(&exp.id).copied().unwrap_or(50.0),
                    "userOverridden": false
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"personaId":persona_id,"scores":scores,"source":"ai"}))
    })())
}

fn strip_llm_json_fence(value: &str) -> &str {
    let trimmed = value.trim();
    if let Some(rest) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
    {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn import_experiences(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let format = required(&v, "format")?;
        let content = required(&v, "content")?;
        let values = if format == "json" {
            serde_json::from_str::<Vec<Value>>(content)
                .map_err(|e| AppError::Validation(format!("invalid JSON: {e}")))?
        } else {
            vec![
                json!({"title":content.lines().find(|x|!x.trim().is_empty()).unwrap_or("导入经历"),"rawDescription":content,"type":"work"}),
            ]
        };
        let mut rows = Vec::new();
        for (i, x) in values.iter().enumerate() {
            let raw = x
                .get("rawDescription")
                .or_else(|| x.get("raw_description"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let title = x
                .get("title")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| AppError::Validation(format!("row {i}: title is required")))?;
            rows.push(Experience {
                id: stable_id("e", &[title, raw, &i.to_string()]),
                user_id: "default".into(),
                kind: exp_type(x.get("type").and_then(Value::as_str).unwrap_or("work"))?,
                title: title.into(),
                organization: x
                    .get("organization")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                start_date: x
                    .get("startDate")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                end_date: x.get("endDate").and_then(Value::as_str).map(str::to_owned),
                raw_description: raw.into(),
                structured_achievements: str_vec(x.get("structuredAchievements")),
                skills_demonstrated: str_vec(x.get("skillsDemonstrated")),
                industry_tags: str_vec(x.get("industryTags")),
                education_level: education_level(x.get("educationLevel"))?,
                status: ExperienceStatus::Confirmed,
                version: 1,
            });
        }
        let c = connection()?;
        let r = SqliteExperienceRepository::new(&c);
        let count = experiences::import_batch(&r, &rows).map_err(AppError::from)?;
        Ok(json!({"count":count}))
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn import_file(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        use base64::Engine;
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let name = required(&v, "fileName")?;
        let encoded = required(&v, "base64Content")?;
        const MAX_ENCODED: usize = (20usize * 1024 * 1024).div_ceil(3) * 4;
        if encoded.len() > MAX_ENCODED {
            return Err(AppError::Validation("file exceeds 20 MiB".into()));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| AppError::Validation("base64Content is invalid".into()))?;
        let text = crate::infra::documents::import::extract(name, &bytes).map_err(|e| match e {
            crate::infra::documents::import::ImportError::Invalid(x) => AppError::Validation(x),
            crate::infra::documents::import::ImportError::Unsupported(x) => {
                AppError::Unavailable(x)
            }
            crate::infra::documents::import::ImportError::Corrupt(x) => AppError::Validation(x),
        })?;
        let commit = v.get("commit").and_then(Value::as_bool).unwrap_or(true);
        if !commit {
            return Ok(json!({"count":0,"content":text}));
        }
        let imported = import_experiences(Some(
            json!({"format":if name.ends_with(".json"){"json"}else{"text"},"content":text}),
        ));
        match imported {
            Envelope::Ok { data, .. } => {
                let mut out = data;
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("content".into(), Value::String(text));
                }
                Ok(out)
            }
            Envelope::Error { error, .. } => Err(AppError::Validation(error.message)),
        }
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_settings(_payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let c = connection()?;
        let mut s=c.prepare("SELECT name,base_url,default_model,credential_target,enabled FROM provider_configs ORDER BY name")?;
        let rows = s
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, bool>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let store = WindowsCredentialStore;
        let rows = rows
            .into_iter()
            .map(|(name,base_url,default_model,target,enabled)| {
                json!({"name":name,"baseUrl":base_url,"defaultModel":default_model,"enabled":enabled,"hasKey":store.exists(&target).unwrap_or(false)})
            })
            .collect::<Vec<_>>();
        Ok(json!({"providers":rows}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn save_settings(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let providers = v
            .get("providers")
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::Validation("providers array is required".into()))?;
        let c = connection()?;
        let store = WindowsCredentialStore;
        for p in providers {
            let name = required(p, "name")?;
            let base_url = required(p, "baseUrl")?;
            let target = credential_target(name, base_url);
            let config = crate::infra::llm::ProviderConfig {
                provider: name.into(),
                base_url: base_url.into(),
                default_model: required(p, "defaultModel")?.into(),
                credential_target: target,
                enabled: p.get("enabled").and_then(Value::as_bool).unwrap_or(true),
            };
            config
                .validate()
                .map_err(|e| AppError::Validation(e.into()))?;
            let old = c
                .query_row(
                    "SELECT base_url,credential_target FROM provider_configs WHERE name=?1",
                    [name],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?;
            let key = p
                .get("apiKey")
                .and_then(Value::as_str)
                .filter(|x| !x.is_empty());
            if old
                .as_ref()
                .is_some_and(|(url, _)| url.trim_end_matches('/') != base_url.trim_end_matches('/'))
                && key.is_none()
            {
                return Err(AppError::Validation(
                    "endpoint changed; apiKey must be entered again".into(),
                ));
            }
            reconcile_credential_target(
                &store,
                old.as_ref().map(|(_, target)| target.as_str()),
                &config.credential_target,
                key,
            )?;
            c.execute("INSERT INTO provider_configs(name,base_url,default_model,credential_target,enabled) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(name) DO UPDATE SET base_url=excluded.base_url,default_model=excluded.default_model,credential_target=excluded.credential_target,enabled=excluded.enabled,updated_at=CURRENT_TIMESTAMP",rusqlite::params![config.provider,config.base_url,config.default_model,config.credential_target,config.enabled])?;
        }
        Ok(json!({"saved":providers.len()}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn test_llm_connection(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.unwrap_or_else(|| json!({}));
        let c = connection()?;
        let name = v.get("name").and_then(Value::as_str);
        let (provider, url, model, target): (String, String, String, String) = if let Some(name) =
            name
        {
            c.query_row("SELECT name,base_url,default_model,credential_target FROM provider_configs WHERE name=?1 AND enabled=1",[name],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){AppError::NotFound("provider".into())}else{e.into()})?
        } else {
            c.query_row("SELECT name,base_url,default_model,credential_target FROM provider_configs WHERE enabled=1 ORDER BY name LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){AppError::Validation("no enabled provider configured".into())}else{e.into()})?
        };
        let key = WindowsCredentialStore
            .get(&target)
            .map_err(AppError::from)?;
        let cfg = crate::infra::llm::ProviderConfig {
            provider: provider.clone(),
            base_url: url,
            default_model: model,
            credential_target: target,
            enabled: true,
        };
        crate::infra::llm::OpenAiCompatibleProvider::new(cfg, key)
            .map_err(|e| AppError::Validation(e.message))?
            .probe()
            .map_err(|e| AppError::Unavailable(e.message))?;
        Ok(json!({"connected":true,"provider":provider}))
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn parse_jd(jd_text: String) -> Envelope<Value> {
    match crate::application::jobs::parse_jd_locally(&jd_text) {
        Ok(v) => {
            let id = stable_id("j", &[&v.raw_text]);
            let saved=connection().and_then(|c|c.execute("INSERT INTO job_descs(id,raw_text,title,years_of_experience,parsed_skills,industry_tags,education_levels,source) VALUES(?1,?2,?3,?4,?5,?6,?7,'manual') ON CONFLICT(id) DO NOTHING",rusqlite::params![id,v.raw_text,v.title,v.minimum_years.map(|n|n.to_string()),serde_json::to_string(&v.required_skills).unwrap_or_else(|_|"[]".into()),serde_json::to_string(&v.industry_terms).unwrap_or_else(|_|"[]".into()),serde_json::to_string(&v.education_terms).unwrap_or_else(|_|"[]".into())]).map(|_|()).map_err(AppError::from));
            match saved {
                Ok(()) => Envelope::ok(
                    json!({"id":id,"title":v.title,"requiredSkills":v.required_skills,"minimumYears":v.minimum_years,"industryTags":v.industry_terms,"educationLevels":v.education_terms,"rawText":v.raw_text}),
                ),
                Err(e) => Envelope::from(Err::<Value, _>(e)),
            }
        }
        Err(_) => Envelope::error(ErrorCode::Validation, "jdText must not be blank"),
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn collect_job_url(payload: Option<Value>) -> Envelope<Value> {
    let url = match payload
        .as_ref()
        .and_then(|v| v.get("url"))
        .and_then(Value::as_str)
    {
        Some(v) => v,
        None => {
            return Envelope::from(Err::<Value, _>(AppError::Validation(
                "url is required".into(),
            )))
        }
    };
    if crate::infra::http::UrlCollectionRequest::new(url).is_err() {
        return Envelope::ok(json!({"manualFallbackRequired":true,"reason":"UnsafeOrInvalidUrl"}));
    }
    let open_token = issue_open_token(url);
    if crate::infra::skills::is_builtin_resource_url(url) {
        return Envelope::ok(json!({
            "manualFallbackRequired": false,
            "finalUrl": url,
            "openToken": open_token,
            "trustedLearningResource": true
        }));
    }
    match crate::infra::http::collect(url) {
        crate::infra::http::CollectionOutcome::Collected { final_url, text } => Envelope::ok(
            json!({"manualFallbackRequired":false,"finalUrl":final_url,"text":text,"openToken":open_token}),
        ),
        crate::infra::http::CollectionOutcome::ManualInputRequired { reason, .. } => Envelope::ok(
            json!({"manualFallbackRequired":true,"reason":format!("{reason:?}"),"openToken":open_token}),
        ),
    }
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn list_jobs(_payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let c = connection()?;
        let mut s=c.prepare("SELECT id,raw_text,title,company,years_of_experience,parsed_skills,source,industry_tags,education_levels FROM job_descs ORDER BY created_at DESC")?;
        let rows=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"rawText":r.get::<_,String>(1)?,"title":r.get::<_,Option<String>>(2)?,"company":r.get::<_,Option<String>>(3)?,"yearsOfExperience":r.get::<_,Option<String>>(4)?,"parsedSkills":serde_json::from_str::<Value>(&r.get::<_,Option<String>>(5)?.unwrap_or_else(||"[]".into())).unwrap_or(json!([])),"source":r.get::<_,Option<String>>(6)?,"industryTags":serde_json::from_str::<Value>(&r.get::<_,String>(7)?).unwrap_or(json!([])),"educationLevels":serde_json::from_str::<Value>(&r.get::<_,String>(8)?).unwrap_or(json!([]))})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!(rows))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn delete_job(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        if c.execute(
            "DELETE FROM job_descs WHERE id=?1",
            [required(&v, "jobDescId")?],
        )? == 0
        {
            return Err(AppError::NotFound("job".into()));
        }
        Ok(json!({"deleted":true}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn match_job(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        use crate::domain::jobs::{score_match, ParsedJob};
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let jid = required(&v, "jobDescId")?;
        let pid = required(&v, "personaId")?;
        let c = connection()?;
        let (skills,years,raw,job_industry,job_education):(String,Option<String>,String,String,String)=c.query_row("SELECT COALESCE(parsed_skills,'[]'),years_of_experience,raw_text,COALESCE(industry_tags,'[]'),COALESCE(education_levels,'[]') FROM job_descs WHERE id=?1",[jid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){AppError::NotFound("job".into())}else{e.into()})?;
        let persona = SqlitePersonaRepository::new(&c)
            .get(pid)?
            .ok_or_else(|| AppError::NotFound("persona".into()))?;
        let parsed = crate::application::jobs::parse_jd_locally(&raw)
            .map_err(|_| AppError::Validation("stored job description is blank".into()))?;
        let persisted_job_industry =
            serde_json::from_str::<Vec<String>>(&job_industry).unwrap_or_default();
        let persisted_job_education =
            serde_json::from_str::<Vec<String>>(&job_education).unwrap_or_default();
        let job = ParsedJob {
            required_skills: serde_json::from_str(&skills).unwrap_or_default(),
            minimum_years: years.and_then(|x| x.parse().ok()),
            industry_terms: if persisted_job_industry.is_empty() {
                parsed.industry_terms
            } else {
                persisted_job_industry
            },
            education_terms: if persisted_job_education.is_empty() {
                parsed.education_terms
            } else {
                persisted_job_education
            },
            raw_text: raw,
            ..Default::default()
        };
        let candidate_experiences =
            SqliteExperienceRepository::new(&c).list_confirmed(&persona.user_id)?;
        let candidate_industry_persisted = candidate_experiences
            .iter()
            .any(|e| !e.industry_tags.is_empty());
        let candidate_education_persisted = candidate_experiences
            .iter()
            .any(|e| e.education_level.is_some());
        let candidate =
            crate::application::jobs::candidate_evidence(&persona, &candidate_experiences);
        let m = score_match(&job, &candidate);
        let id = stable_id("m", &[jid, pid]);
        let evidence_sources = json!({"jobIndustry":if serde_json::from_str::<Vec<String>>(&job_industry).unwrap_or_default().is_empty(){"legacy_heuristic"}else{"persisted"},"jobEducation":if serde_json::from_str::<Vec<String>>(&job_education).unwrap_or_default().is_empty(){"legacy_heuristic"}else{"persisted"},"candidateIndustry":if candidate_industry_persisted{"persisted"}else{"legacy_heuristic"},"candidateEducation":if candidate_education_persisted{"persisted"}else{"legacy_heuristic"},"candidateSkills":"persisted","candidateExperience":"persisted"});
        let breakdown = json!({"skills":m.skill_score,"experience":m.experience_score,"industry":m.industry_score,"education":m.education_score,"evidenceSources":evidence_sources});
        c.execute("INSERT INTO job_matches(id,persona_id,job_desc_id,match_score,matched_skills,missing_skills,score_breakdown,tracking_status) VALUES(?1,?2,?3,?4,?5,?6,?7,'new') ON CONFLICT(id) DO UPDATE SET match_score=excluded.match_score,matched_skills=excluded.matched_skills,missing_skills=excluded.missing_skills,score_breakdown=excluded.score_breakdown,updated_at=CURRENT_TIMESTAMP",rusqlite::params![id,pid,jid,m.total,serde_json::to_string(&m.matched_skills).unwrap(),serde_json::to_string(&m.missing_skills).unwrap(),breakdown.to_string()])?;
        c.execute("INSERT INTO job_status_events(match_id,from_status,to_status) SELECT ?1,NULL,'new' WHERE NOT EXISTS(SELECT 1 FROM job_status_events WHERE match_id=?1)",[&id])?;
        let (updated_at, version): (String, i64) = c.query_row(
            "SELECT updated_at,version FROM job_matches WHERE id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(
            json!({"id":id,"personaId":pid,"jobDescId":jid,"matchScore":m.total,"matchedSkills":m.matched_skills,"missingSkills":m.missing_skills,"trackingStatus":"new","scoreBreakdown":breakdown,"evidenceSources":evidence_sources,"updatedAt":updated_at,"version":version}),
        )
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_job_matches(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        let mut s=c.prepare("SELECT id,persona_id,job_desc_id,match_score,matched_skills,missing_skills,score_breakdown,tracking_status,updated_at,version FROM job_matches WHERE job_desc_id=?1 ORDER BY updated_at DESC,id DESC")?;
        let rows=s.query_map([required(&v,"jobDescId")?],|r|{let breakdown=serde_json::from_str::<Value>(&r.get::<_,Option<String>>(6)?.unwrap_or_else(||"{}".into())).unwrap_or(json!({}));let evidence=breakdown.get("evidenceSources").cloned().unwrap_or_else(||json!({}));Ok(json!({"id":r.get::<_,String>(0)?,"personaId":r.get::<_,String>(1)?,"jobDescId":r.get::<_,String>(2)?,"matchScore":r.get::<_,u8>(3)?,"matchedSkills":serde_json::from_str::<Value>(&r.get::<_,Option<String>>(4)?.unwrap_or_else(||"[]".into())).unwrap_or(json!([])),"missingSkills":serde_json::from_str::<Value>(&r.get::<_,Option<String>>(5)?.unwrap_or_else(||"[]".into())).unwrap_or(json!([])),"trackingStatus":r.get::<_,String>(7)?,"scoreBreakdown":breakdown,"evidenceSources":evidence,"updatedAt":r.get::<_,String>(8)?,"version":r.get::<_,i64>(9)?}))})?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!(rows))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_match_status(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        use crate::{application::jobs::transition, domain::jobs::JobStatus};
        fn st(v: &str) -> Option<JobStatus> {
            Some(match v {
                "new" => JobStatus::New,
                "interested" => JobStatus::Interested,
                "applied" => JobStatus::Applied,
                "interviewing" => JobStatus::Interviewing,
                "offered" => JobStatus::Offered,
                "rejected" => JobStatus::Rejected,
                "ghosted" => JobStatus::Ghosted,
                "accepted" => JobStatus::Accepted,
                "declined" => JobStatus::Declined,
                _ => return None,
            })
        }
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let id = required(&v, "matchId")?;
        let next = required(&v, "status")?;
        let expected = v
            .get("expectedVersion")
            .and_then(Value::as_i64)
            .filter(|n| *n > 0)
            .ok_or_else(|| AppError::Validation("expectedVersion is required".into()))?;
        let mut c = connection()?;
        let tx = c.transaction()?;
        let (current, version): (String, i64) = tx
            .query_row(
                "SELECT tracking_status,version FROM job_matches WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    AppError::NotFound("match".into())
                } else {
                    e.into()
                }
            })?;
        if version != expected {
            return Err(AppError::Conflict("stale job match version".into()));
        }
        transition(
            st(&current).ok_or_else(|| AppError::Internal)?,
            st(next).ok_or_else(|| AppError::Validation("invalid status".into()))?,
        )
        .map_err(|_| AppError::Conflict("invalid status transition".into()))?;
        let updated=tx.execute("UPDATE job_matches SET tracking_status=?1,version=version+1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND version=?3",rusqlite::params![next,id,expected])?;
        if updated != 1 {
            return Err(AppError::Conflict("stale job match version".into()));
        }
        tx.execute(
            "INSERT INTO job_status_events(match_id,from_status,to_status) VALUES(?1,?2,?3)",
            rusqlite::params![id, current, next],
        )?;
        let event_id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(json!({"id":id,"trackingStatus":next,"eventId":event_id,"version":expected+1}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_job_status_events(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let match_id = required(&v, "matchId")?;
        let c = connection()?;
        if !c.query_row(
            "SELECT EXISTS(SELECT 1 FROM job_matches WHERE id=?1)",
            [match_id],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(AppError::NotFound("match".into()));
        }
        let mut s=c.prepare("SELECT id,match_id,from_status,to_status,changed_at FROM job_status_events WHERE match_id=?1 ORDER BY id")?;
        let rows=s.query_map([match_id],|r|Ok(json!({"id":r.get::<_,i64>(0)?.to_string(),"matchId":r.get::<_,String>(1)?,"fromStatus":r.get::<_,Option<String>>(2)?,"toStatus":r.get::<_,String>(3)?,"changedAt":r.get::<_,String>(4)?})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!(rows))
    })())
}

fn skill_json(s: &crate::domain::skills::Skill) -> Value {
    json!({"skillId":s.id,"id":s.id,"name":s.name,"category":s.category,"description":s.description,"aliases":s.aliases,"prerequisiteSkillIds":s.prerequisites,"prerequisites":s.prerequisites,"level":s.level,"resources":s.resources.iter().map(|r|json!({"resourceId":stable_id("sr",&[&s.id,&r.url]),"skillId":s.id,"kind":r.kind,"title":r.title,"source":r.source,"url":r.url,"estimatedHours":r.estimated_hours})).collect::<Vec<_>>()})
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_skill_graph(_payload: Option<Value>) -> Envelope<Value> {
    match crate::infra::skills::builtin_skills() {
        Ok(v) => Envelope::ok(json!(v.iter().map(skill_json).collect::<Vec<_>>())),
        Err(_) => Envelope::error(ErrorCode::Internal, "built-in skill catalog invalid"),
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn search_skills(payload: Option<Value>) -> Envelope<Value> {
    let Some(v) = payload else {
        return Envelope::error(ErrorCode::Validation, "payload is required");
    };
    let Ok(q) = required(&v, "query") else {
        return Envelope::error(ErrorCode::Validation, "query is required");
    };
    match crate::infra::skills::builtin_skills() {
        Ok(v) => {
            let q = q.to_lowercase();
            Envelope::ok(json!(v
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&q)
                    || s.aliases.iter().any(|a| a.to_lowercase().contains(&q)))
                .map(skill_json)
                .collect::<Vec<_>>()))
        }
        Err(_) => Envelope::error(ErrorCode::Internal, "built-in skill catalog invalid"),
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_skill_resources(payload: Option<Value>) -> Envelope<Value> {
    let Some(v) = payload else {
        return Envelope::error(ErrorCode::Validation, "payload is required");
    };
    let Ok(id) = required(&v, "skillId") else {
        return Envelope::error(ErrorCode::Validation, "skillId is required");
    };
    match crate::infra::skills::builtin_skills(){Ok(v)=>match v.into_iter().find(|s|s.id==id){Some(s)=>Envelope::ok(json!(s.resources.iter().map(|r|json!({"resourceId":stable_id("sr",&[&s.id,&r.url]),"skillId":s.id,"kind":r.kind,"title":r.title,"source":r.source,"url":r.url,"estimatedHours":r.estimated_hours})).collect::<Vec<_>>())),None=>Envelope::error(ErrorCode::NotFound,"skill not found")},Err(_)=>Envelope::error(ErrorCode::Internal,"built-in skill catalog invalid")}
}
fn normalized_learning_items(c: &Connection, path_id: &str) -> Result<Vec<Value>, AppError> {
    let mut s=c.prepare("SELECT i.id,i.skill_id,i.title,i.resource_url,i.resource_kind,i.estimated_hours,i.status,i.source,i.version,i.completion_note,lc.experience_id FROM learning_items i LEFT JOIN learning_conversions lc ON lc.item_id=i.id WHERE i.path_id=?1 ORDER BY i.id")?;
    let rows=s.query_map([path_id],|r|Ok(json!({"itemId":r.get::<_,String>(0)?,"skillId":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"resourceUrl":r.get::<_,Option<String>>(3)?,"resourceKind":r.get::<_,String>(4)?,"estimatedHours":r.get::<_,i64>(5)?,"status":r.get::<_,String>(6)?,"source":r.get::<_,String>(7)?,"version":r.get::<_,i64>(8)?,"completionNote":r.get::<_,Option<String>>(9)?,"convertedExperienceId":r.get::<_,Option<String>>(10)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(rows)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_learning_paths_by_source(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let c = connection()?;
        let source = payload
            .as_ref()
            .and_then(|v| v.get("sourceType"))
            .and_then(Value::as_str);
        if source.is_some_and(|v| {
            !["skill_graph", "job_gap", "manual", "llm_with_skill_graph"].contains(&v)
        }) {
            return Err(AppError::Validation("invalid sourceType".into()));
        }
        let mut s=c.prepare("SELECT id,persona_id,target_gap,source_type,status,context_json,version FROM learning_paths WHERE (?1 IS NULL OR source_type=?1) ORDER BY created_at DESC,id DESC")?;
        let metadata = s
            .query_map([source], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(s);
        let rows=metadata.into_iter().map(|(id,persona,target,source,status,context,version)|{let context_value=serde_json::from_str::<Value>(&context).map_err(|_|AppError::Internal)?;let skill_id=context_value.get("skillId").and_then(Value::as_str).unwrap_or("");let guidance=context_value.get("guidance").and_then(Value::as_str).unwrap_or("");let items=normalized_learning_items(&c,&id)?;Ok(json!({"pathId":id,"personaId":persona,"targetGap":target,"skillId":skill_id,"sourceType":source,"guidance":guidance,"status":status,"context":context_value,"version":version,"items":items}))}).collect::<Result<Vec<_>,AppError>>()?;
        Ok(json!(rows))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_learning_path(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let query = required(&v, "skill")?.to_lowercase();
        let skill = crate::infra::skills::builtin_skills()
            .map_err(|_| AppError::Internal)?
            .into_iter()
            .find(|s| {
                s.id.to_lowercase() == query
                    || s.name.to_lowercase() == query
                    || s.aliases.iter().any(|a| a.to_lowercase() == query)
            })
            .ok_or_else(|| AppError::NotFound("skill".into()))?;
        if skill.resources.len() < 3 {
            return Err(AppError::Validation(
                "trusted catalog has fewer than three resources for this gap".into(),
            ));
        }
        let mut c = connection()?;
        let persona_id = if let Some(id) = v.get("personaId").and_then(Value::as_str) {
            id.to_owned()
        } else {
            c.query_row(
                "SELECT id FROM personas ORDER BY is_default DESC,created_at,id LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    AppError::Validation("personaId is required when no persona exists".into())
                } else {
                    e.into()
                }
            })?
        };
        let guidance = llm_generate(
            &c,
            format!(
                "为技能“{}”制定三步可执行学习路径。仅返回简洁中文，不虚构证书或成果。",
                skill.name
            ),
        )?;
        let path_id = stable_id("lp", &[&persona_id, &skill.id, &guidance]);
        let items=skill.resources.iter().take(3).map(|r|json!({"itemId":stable_id("li",&[&path_id,&skill.id,&r.url]),"skillId":skill.id,"title":r.title,"resourceUrl":r.url,"resourceKind":r.kind,"estimatedHours":r.estimated_hours,"status":"pending","source":r.source,"version":1,"completionNote":Value::Null,"convertedExperienceId":Value::Null})).collect::<Vec<_>>();
        let context = json!({"skillId":skill.id,"skillName":skill.name,"requestedSkill":query,"guidance":guidance});
        let tx = c.transaction()?;
        tx.execute("INSERT INTO learning_paths(id,persona_id,target_gap,items,source_type,status,context_json,version) VALUES(?1,?2,?3,?4,'llm_with_skill_graph','active',?5,1)",rusqlite::params![path_id,persona_id,skill.name,serde_json::to_string(&items).map_err(|_|AppError::Internal)?,context.to_string()])?;
        for item in &items {
            tx.execute("INSERT INTO learning_items(id,path_id,skill_id,title,resource_url,estimated_hours,status,source,resource_kind,version) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,?8,1)",rusqlite::params![item["itemId"].as_str(),path_id,skill.id,item["title"].as_str(),item["resourceUrl"].as_str(),item["estimatedHours"].as_u64(),item["source"].as_str(),item["resourceKind"].as_str()])?;
        }
        tx.commit()?;
        Ok(
            json!({"pathId":path_id,"personaId":persona_id,"targetGap":skill.name,"skillId":skill.id,"sourceType":"llm_with_skill_graph","guidance":guidance,"context":context,"status":"active","version":1,"items":items}),
        )
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn reframe_resume(payload: Option<Value>) -> Envelope<Value> {
    reframe_resume_with_cancel(payload, &crate::infra::llm::CancellationToken::default())
}
pub(crate) fn reframe_resume_with_cancel(
    payload: Option<Value>,
    cancel: &crate::infra::llm::CancellationToken,
) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let match_id = required(&v, "matchId")?;
        let c = connection()?;
        let (jd,persona):(String,String)=c.query_row("SELECT j.raw_text,m.persona_id FROM job_matches m JOIN job_descs j ON j.id=m.job_desc_id WHERE m.id=?1",[match_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|if matches!(e,rusqlite::Error::QueryReturnedNoRows){AppError::NotFound("match".into())}else{e.into()})?;
        let exps = SqliteExperienceRepository::new(&c).list_confirmed("default")?;
        let mut rows = Vec::new();
        for exp in exps {
            let reframed=generate_for_persona(Some(&persona),format!("岗位JD：\n{}\n\n候选人原始经历：\n{}\n\n在不增加任何事实、数字、技能或职责的前提下，重述该经历以突出与岗位相关的部分。只返回重述正文。",jd,exp.raw_description),cancel)?;
            let id = stable_id("r", &[match_id, &exp.id]);
            c.execute("INSERT INTO job_match_experience_reframes(id,job_match_id,experience_id,original_summary,reframed_summary,reframing_strategy) VALUES(?1,?2,?3,?4,?5,'fact_preserving_llm') ON CONFLICT(job_match_id,experience_id) DO UPDATE SET reframed_summary=excluded.reframed_summary,reframing_strategy=excluded.reframing_strategy",rusqlite::params![id,match_id,exp.id,exp.raw_description,reframed])?;
            rows.push(json!({"id":id,"experienceId":exp.id,"originalSummary":exp.raw_description,"reframedSummary":reframed}));
        }
        Ok(json!({"matchId":match_id,"personaId":persona,"reframes":rows}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_reframe_results(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        let mut s=c.prepare("SELECT id,job_match_id,experience_id,original_summary,reframed_summary,reframing_strategy FROM job_match_experience_reframes WHERE job_match_id=?1 ORDER BY created_at")?;
        let rows=s.query_map([required(&v,"matchId")?],|r|Ok(json!({"id":r.get::<_,String>(0)?,"matchId":r.get::<_,String>(1)?,"experienceId":r.get::<_,String>(2)?,"originalSummary":r.get::<_,String>(3)?,"reframedSummary":r.get::<_,String>(4)?,"reframingStrategy":r.get::<_,Option<String>>(5)?})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!({"count":rows.len(),"reframes":rows}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_reframe(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let id = required(&v, "reframeId")?;
        let text = required(&v, "reframedSummary")?;
        let c = connection()?;
        if c.execute(
            "UPDATE job_match_experience_reframes SET reframed_summary=?1 WHERE id=?2",
            rusqlite::params![text, id],
        )? == 0
        {
            return Err(AppError::NotFound("reframe".into()));
        }
        Ok(json!({"id":id,"reframedSummary":text}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn reset_reframe(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        if c.execute(
            "DELETE FROM job_match_experience_reframes WHERE id=?1",
            [required(&v, "reframeId")?],
        )? == 0
        {
            return Err(AppError::NotFound("reframe".into()));
        }
        Ok(json!({"deleted":true}))
    })())
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn list_custom_skills(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.unwrap_or_else(|| json!({}));
        let owner = v
            .get("ownerId")
            .and_then(Value::as_str)
            .unwrap_or("default");
        let c = connection()?;
        let mut s=c.prepare("SELECT id,owner_id,name,category,description,aliases,prerequisites,level,resources FROM custom_skills WHERE owner_id=?1 ORDER BY name COLLATE NOCASE")?;
        let rows=s.query_map([owner],|r|Ok(json!({"id":r.get::<_,String>(0)?,"ownerId":r.get::<_,String>(1)?,"name":r.get::<_,String>(2)?,"category":r.get::<_,String>(3)?,"description":r.get::<_,String>(4)?,"aliases":serde_json::from_str::<Value>(&r.get::<_,String>(5)?).unwrap_or(json!([])),"prerequisites":serde_json::from_str::<Value>(&r.get::<_,String>(6)?).unwrap_or(json!([])),"level":r.get::<_,u8>(7)?,"resources":serde_json::from_str::<Value>(&r.get::<_,String>(8)?).unwrap_or(json!([]))})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!(rows))
    })())
}
fn custom_skill_value(v: &Value) -> Result<CustomSkillFields<'_>, AppError> {
    let id = required(v, "id")?;
    let owner = v
        .get("ownerId")
        .and_then(Value::as_str)
        .unwrap_or("default");
    let name = required(v, "name")?;
    let category = required(v, "category")?;
    let level = v.get("level").and_then(Value::as_u64).unwrap_or(1) as u8;
    if !(1..=3).contains(&level) {
        return Err(AppError::Validation("level must be 1..3".into()));
    }
    Ok((
        id,
        owner,
        name,
        category,
        v.get("aliases").cloned().unwrap_or(json!([])).to_string(),
        v.get("prerequisites")
            .cloned()
            .unwrap_or(json!([]))
            .to_string(),
        level,
        v.get("resources").cloned().unwrap_or(json!([])).to_string(),
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn create_custom_skill(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let s = v.get("skill").unwrap_or(&v);
        let (id, owner, name, category, aliases, pre, level, res) = custom_skill_value(s)?;
        let c = connection()?;
        c.execute("INSERT INTO custom_skills(id,owner_id,name,category,description,aliases,prerequisites,level,resources) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",rusqlite::params![id,owner,name,category,s.get("description").and_then(Value::as_str).unwrap_or(""),aliases,pre,level,res]).map_err(|e|if matches!(e,rusqlite::Error::SqliteFailure(_,Some(ref x))if x.contains("UNIQUE")){AppError::Conflict("custom skill name already exists".into())}else{e.into()})?;
        Ok(json!({"id":id}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_custom_skill(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let s = v.get("skill").unwrap_or(&v);
        let (id, owner, name, category, aliases, pre, level, res) = custom_skill_value(s)?;
        let c = connection()?;
        if c.execute("UPDATE custom_skills SET name=?1,category=?2,description=?3,aliases=?4,prerequisites=?5,level=?6,resources=?7,updated_at=CURRENT_TIMESTAMP WHERE id=?8 AND owner_id=?9",rusqlite::params![name,category,s.get("description").and_then(Value::as_str).unwrap_or(""),aliases,pre,level,res,id,owner])?==0{return Err(AppError::NotFound("custom skill".into()));}
        Ok(json!({"id":id}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn delete_custom_skill(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        if c.execute(
            "DELETE FROM custom_skills WHERE id=?1 AND owner_id=?2",
            rusqlite::params![
                required(&v, "skillId")?,
                v.get("ownerId")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
            ],
        )? == 0
        {
            return Err(AppError::NotFound("custom skill".into()));
        }
        Ok(json!({"deleted":true}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn simulate_skill_what_if(payload: Option<Value>) -> Envelope<Value> {
    let Some(v) = payload else {
        return Envelope::error(ErrorCode::Validation, "payload is required");
    };
    let parse = |key| str_vec(v.get(key));
    let r = crate::domain::skills::simulate_skills(
        &parse("requiredSkills"),
        &parse("currentSkills"),
        &parse("hypotheticalSkills"),
    );
    Envelope::ok(
        json!({"baselineScore":r.baseline_score,"simulatedScore":r.simulated_score,"delta":r.delta,"addedSkills":r.added_skills,"remainingMissing":r.remaining_missing}),
    )
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_learning_progress(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let id = required(&v, "itemId")?;
        let expected = v
            .get("expectedVersion")
            .and_then(Value::as_i64)
            .filter(|n| *n > 0)
            .ok_or_else(|| AppError::Validation("expectedVersion is required".into()))?;
        let next = required(&v, "status")?;
        if !["pending", "in_progress", "completed", "skipped"].contains(&next) {
            return Err(AppError::Validation("invalid learning status".into()));
        }
        if next == "completed"
            && v.get("completionNote")
                .and_then(Value::as_str)
                .is_none_or(|x| x.trim().is_empty())
        {
            return Err(AppError::Validation("completionNote is required".into()));
        }
        let mut c = connection()?;
        let tx = c.transaction()?;
        let (current, version): (String, i64) = tx
            .query_row(
                "SELECT status,version FROM learning_items WHERE id=?1 AND path_id=?2",
                rusqlite::params![id, required(&v, "pathId")?],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    AppError::NotFound("learning item".into())
                } else {
                    e.into()
                }
            })?;
        if version != expected {
            return Err(AppError::Conflict("stale learning item version".into()));
        }
        let valid = current == next
            || matches!(
                (current.as_str(), next),
                ("pending", "in_progress" | "completed" | "skipped")
                    | ("in_progress", "completed" | "skipped")
            );
        if !valid {
            return Err(AppError::Conflict(
                "invalid learning status transition".into(),
            ));
        }
        let n=tx.execute("UPDATE learning_items SET status=?1,completion_note=COALESCE(?2,completion_note),version=version+1,updated_at=CURRENT_TIMESTAMP WHERE id=?3 AND version=?4",rusqlite::params![next,v.get("completionNote").and_then(Value::as_str),id,expected])?;
        if n != 1 {
            return Err(AppError::Conflict("stale learning item version".into()));
        }
        tx.execute(
            "UPDATE learning_paths SET version=version+1,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [required(&v, "pathId")?],
        )?;
        let path_version: i64 = tx.query_row(
            "SELECT version FROM learning_paths WHERE id=?1",
            [required(&v, "pathId")?],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(
            json!({"itemId":id,"status":next,"version":expected+1,"pathVersion":path_version,"completionNote":v.get("completionNote").and_then(Value::as_str)}),
        )
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn complete_learning_to_experience(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let item = required(&v, "itemId")?;
        let exp = required(&v, "experienceId")?;
        let edited_title = required(&v, "title")?.trim();
        let edited_organization = required(&v, "organization")?.trim();
        let edited_raw = required(&v, "rawDescription")?.trim();
        if edited_title.len() > 200 || edited_organization.len() > 200 || edited_raw.len() > 100_000
        {
            return Err(AppError::Validation(
                "learning experience fields exceed limits".into(),
            ));
        }
        let mut c = connection()?;
        let tx = c.transaction()?;
        let (skill, title, note, status): (String, String, Option<String>, String) = tx
            .query_row(
                "SELECT skill_id,title,completion_note,status FROM learning_items WHERE id=?1",
                [item],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    AppError::NotFound("learning item".into())
                } else {
                    e.into()
                }
            })?;
        if status != "completed" {
            return Err(AppError::Conflict("learning item is not completed".into()));
        }
        let note = note
            .filter(|x| !x.trim().is_empty())
            .ok_or_else(|| AppError::Validation("completion note is required".into()))?;
        if tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM learning_conversions WHERE item_id=?1)",
            [item],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(AppError::Conflict("learning item already converted".into()));
        }
        // Conversion creates an editable draft. Confirmation/discard continues
        // through the ordinary experience lifecycle and optimistic versioning.
        tx.execute("INSERT INTO experiences(id,user_id,type,title,organization,raw_description,skills_demonstrated,status,version) VALUES(?1,'default','project',?2,?3,?4,?5,'draft',1)",rusqlite::params![exp,edited_title,edited_organization,edited_raw,json!([skill]).to_string()])?;
        append_experience_revision(&tx, exp, "create").map_err(AppError::from)?;
        let path_id: String = tx.query_row(
            "SELECT path_id FROM learning_items WHERE id=?1",
            [item],
            |r| r.get(0),
        )?;
        tx.execute("INSERT INTO learning_conversions(item_id,experience_id,source_item_id,source_path_id,source_skill_id,source_title,completion_note_snapshot) VALUES(?1,?2,?1,?3,?4,?5,?6)",rusqlite::params![item,exp,path_id,skill,title,note])?;
        let conversion_id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(
            json!({"experienceId":exp,"converted":true,"conversionId":conversion_id.to_string(),"sourceSnapshot":{"itemId":item,"pathId":path_id,"skillId":skill,"title":title,"completionNote":note},"draft":{"id":exp,"version":1,"status":"draft","title":edited_title,"organization":edited_organization,"rawDescription":edited_raw,"sourceLearningTitle":title,"completionNote":note}}),
        )
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn reset_fit_score(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let c = connection()?;
        let er = SqliteExperienceRepository::new(&c);
        let pr = SqlitePersonaRepository::new(&c);
        let w = fit_scores::reset_override(
            &er,
            &pr,
            required(&v, "personaId")?,
            required(&v, "experienceId")?,
        )
        .map_err(AppError::from)?;
        Ok(
            json!({"experienceId":w.experience_id,"relevanceScore":w.relevance_score*100.0,"userOverridden":w.user_overridden}),
        )
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn delete_provider(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let name = required(&v, "name")?;
        let mut c = connection()?;
        let tx = c.transaction()?;
        let linked: i64 = tx.query_row(
            "SELECT count(*) FROM personas WHERE preferred_model=?1 OR preferred_model LIKE ?2",
            rusqlite::params![name, format!("{name}/%")],
            |r| r.get(0),
        )?;
        if linked > 0 {
            return Err(AppError::Conflict(format!(
                "provider is preferred by {linked} persona(s); change their preferred model first"
            )));
        }
        let target = tx
            .query_row(
                "SELECT credential_target FROM provider_configs WHERE name=?1",
                [name],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::NotFound("provider".into()))?;
        WindowsCredentialStore
            .delete(&target)
            .map_err(AppError::from)?;
        tx.execute("DELETE FROM provider_configs WHERE name=?1", [name])?;
        tx.commit()?;
        Ok(json!({"deleted":true,"name":name}))
    })())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn open_external_url(payload: Option<Value>) -> Envelope<Value> {
    result((|| {
        let v = payload.ok_or_else(|| AppError::Validation("payload is required".into()))?;
        let url = consume_open_token(required(&v, "token")?)?;
        #[cfg(windows)]
        {
            std::process::Command::new("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", &url])
                .spawn()
                .map_err(|_| AppError::Unavailable("failed to open system browser".into()))?;
        }
        #[cfg(not(windows))]
        {
            return Err(AppError::Unavailable(
                "external browser integration is only available in the Windows build".into(),
            ));
        }
        #[allow(unreachable_code)]
        Ok(json!({"opened":true,"url":url}))
    })())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn value<T: serde::Serialize>(v: T) -> Value {
        serde_json::to_value(v).unwrap()
    }
    #[derive(Default)]
    struct FakeSecrets(std::cell::RefCell<std::collections::BTreeMap<String, String>>);
    impl SecretStore for FakeSecrets {
        fn put(&self, p: &str, s: &str) -> Result<(), crate::application::ports::ApplicationError> {
            self.0.borrow_mut().insert(p.into(), s.into());
            Ok(())
        }
        fn get(&self, p: &str) -> Result<String, crate::application::ports::ApplicationError> {
            self.0.borrow().get(p).cloned().ok_or_else(|| {
                crate::application::ports::ApplicationError::NotFound("credential".into())
            })
        }
        fn exists(&self, p: &str) -> Result<bool, crate::application::ports::ApplicationError> {
            Ok(self.0.borrow().contains_key(p))
        }
        fn delete(&self, p: &str) -> Result<(), crate::application::ports::ApplicationError> {
            self.0.borrow_mut().remove(p);
            Ok(())
        }
    }
    #[test]
    fn credential_target_migration_is_isolated_and_never_plaintext_config() {
        let s = FakeSecrets::default();
        reconcile_credential_target(&s, None, "target/a", Some("secret-a")).unwrap();
        reconcile_credential_target(&s, None, "target/b", Some("secret-b")).unwrap();
        assert_eq!(s.get("target/a").unwrap(), "secret-a");
        assert_eq!(s.get("target/b").unwrap(), "secret-b");
        assert!(reconcile_credential_target(&s, Some("target/a"), "target/new", None).is_err());
        reconcile_credential_target(&s, Some("target/a"), "target/new", Some("replacement"))
            .unwrap();
        assert!(!s.exists("target/a").unwrap());
        assert_eq!(s.get("target/new").unwrap(), "replacement");
    }
    #[test]
    fn all_contract_slots_are_registered() {
        let source: Value =
            serde_json::from_str(include_str!("../../../contracts/commands/v1/commands.json"))
                .unwrap();
        let expected = source["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v[0].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(CONTRACT_COMMANDS.len(), 33);
        assert_eq!(expected, CONTRACT_COMMANDS);
    }
    #[test]
    fn v3_freezes_structure_experience_preview_and_61_commands() {
        let v: Value =
            serde_json::from_str(include_str!("../../../contracts/commands/v3/commands.json"))
                .unwrap();
        assert_eq!(v["topLevelCommandCount"], 61);
        assert_eq!(v["topLevelCommandsAdded"][0][0], "previewResume");
        assert_eq!(v["topLevelCommandsAdded"][1][0], "getJobStatusEvents");
        assert_eq!(v["topLevelCommandsAdded"][2][0], "checkUpdate");
        let op = &v["backgroundOperations"]["structure_experience"];
        assert_eq!(op["request"]["required"], json!(["rawDescription"]));
        assert_eq!(
            op["response"]["required"],
            json!(["draft", "promptVersion", "provider", "model", "warnings"])
        );
        assert_eq!(op["response"]["additionalProperties"], false);
        assert_eq!(
            op["response"]["draft"]["required"],
            json!([
                "type",
                "title",
                "organization",
                "startDate",
                "endDate",
                "rawDescription",
                "structuredAchievements",
                "skillsDemonstrated",
                "industryTags",
                "educationLevel",
                "status"
            ])
        );
        assert_eq!(op["response"]["draft"]["additionalProperties"], false);
        assert_eq!(op["response"]["draft"]["status"], "draft");
        assert_eq!(op["sideEffects"], "none");
        let invalid = serde_json::to_value(structure_experience_with_cancel(
            None,
            &crate::infra::llm::CancellationToken::default(),
        ))
        .unwrap();
        assert_eq!(invalid["error"]["code"], "VALIDATION");
        assert_eq!(
            serde_json::to_value(preview_resume(None)).unwrap()["error"]["code"],
            "VALIDATION"
        )
    }
    #[test]
    fn structure_experience_interface_path_has_no_database_side_effects() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("structure-runtime.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let counts = || {
            let c = Connection::open(&path).unwrap();
            ["experiences", "role_experience_weights", "resume_versions"]
                .into_iter()
                .map(|table| {
                    c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| {
                        r.get::<_, u32>(0)
                    })
                    .unwrap()
                })
                .collect::<Vec<_>>()
        };
        let before = counts();
        let output = r#"{"type":"work","title":"Engineer","organization":null,"startDate":"2024-01","endDate":null,"structuredAchievements":["Built service"],"skillsDemonstrated":["Rust"]}"#;
        let value = serde_json::to_value(structure_experience_with_generator(
            Some(json!({"rawDescription":"Built service with Rust"})),
            &crate::infra::llm::CancellationToken::default(),
            |_, _, _| {
                Ok((
                    crate::domain::llm::GenerationResult {
                        text: output.into(),
                        provider: "fake".into(),
                        model: "fixture".into(),
                    },
                    false,
                ))
            },
        ))
        .unwrap();
        assert_eq!(value["success"], true);
        assert_eq!(value["data"]["draft"]["status"], "draft");
        assert_eq!(counts(), before);
        std::env::remove_var("CAREERCRAFT_DB_PATH")
    }
    #[test]
    fn get_job_matches_has_named_shape_evidence_and_stable_updated_order() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("match-shape.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p','u','P',1,'{}','[]',5)",[]).unwrap();
        c.execute("INSERT INTO job_descs(id,raw_text) VALUES('j','job')", [])
            .unwrap();
        let evidence = json!({"jobIndustry":"persisted","jobEducation":"persisted","candidateIndustry":"persisted","candidateEducation":"persisted","candidateSkills":"persisted","candidateExperience":"persisted"});
        for id in ["a", "b"] {
            let breakdown = json!({"skills":50,"experience":25,"industry":15,"education":10,"evidenceSources":evidence});
            c.execute("INSERT INTO job_matches(id,persona_id,job_desc_id,match_score,matched_skills,missing_skills,score_breakdown,tracking_status,updated_at) VALUES(?1,'p','j',100,'[]','[]',?2,'new','2026-01-01')",rusqlite::params![id,breakdown.to_string()]).unwrap();
        }
        drop(c);
        let value = value(get_job_matches(Some(json!({"jobDescId":"j"}))));
        let items = value["data"].as_array().unwrap();
        assert_eq!(items[0]["id"], "b");
        for item in items {
            for key in [
                "id",
                "personaId",
                "jobDescId",
                "matchScore",
                "matchedSkills",
                "missingSkills",
                "trackingStatus",
                "scoreBreakdown",
                "evidenceSources",
                "updatedAt",
            ] {
                assert!(item.get(key).is_some(), "missing {key}")
            }
            assert_eq!(
                item["evidenceSources"],
                item["scoreBreakdown"]["evidenceSources"]
            )
        }
        std::env::remove_var("CAREERCRAFT_DB_PATH")
    }
    #[test]
    fn external_url_token_is_bound_and_single_use() {
        let token = issue_open_token("https://jobs.example/role");
        assert_eq!(
            consume_open_token(&token).unwrap(),
            "https://jobs.example/role"
        );
        assert!(consume_open_token(&token).is_err());
    }
    #[test]
    fn bundled_learning_resource_gets_open_token_without_page_collection() {
        let url = "https://www.nngroup.com/articles/user-research-methods/";
        let response = value(collect_job_url(Some(json!({"url": url}))));
        assert_eq!(response["success"], true);
        assert_eq!(response["data"]["trustedLearningResource"], true);
        assert_eq!(response["data"]["manualFallbackRequired"], false);
        assert!(response["data"].get("text").is_none());
        let token = response["data"]["openToken"].as_str().unwrap();
        assert_eq!(consume_open_token(token).unwrap(), url);
    }
    #[test]
    fn local_commands_cover_success_validation_and_domain_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("commands.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let p = value(create_persona(Some(
            json!({"id":"p1","name":"技术","capabilityWeights":{"rust":1.0}}),
        )));
        assert_eq!(p["success"], true);
        let e = value(save_experience(Some(
            json!({"newId":"e1","type":"work","title":"工程师","rawDescription":"Rust service","skillsDemonstrated":["rust"],"status":"confirmed"}),
        )));
        assert_eq!(e["success"], true);
        assert_eq!(
            value(get_experiences(None))["data"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            value(get_personas(None))["data"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            value(get_experiences_with_fit_score(Some(
                json!({"personaId":"p1"})
            )))["success"],
            true
        );
        assert_eq!(
            value(update_fit_score(Some(
                json!({"personaId":"p1","experienceId":"e1","score":25})
            )))["success"],
            true
        );
        assert_eq!(
            value(reset_fit_score(Some(
                json!({"personaId":"p1","experienceId":"e1"})
            )))["data"]["userOverridden"],
            false
        );
        assert_eq!(
            value(import_experiences(Some(
                json!({"format":"text","content":"第二段经历"})
            )))["data"]["count"],
            1
        );
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode("文件经历");
        assert_eq!(
            value(import_file(Some(
                json!({"fileName":"经历.txt","base64Content":encoded})
            )))["success"],
            true
        );
        assert_eq!(
            value(save_settings(Some(
                json!({"providers":[{"name":"local","baseUrl":"http://127.0.0.1:9","defaultModel":"test","credentialTarget":"local"}]})
            )))["success"],
            true
        );
        assert_eq!(
            value(get_settings(None))["data"]["providers"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            value(test_llm_connection(Some(json!({"name":"local"}))))["error"]["code"],
            "NOT_FOUND"
        );
        assert_eq!(
            value(delete_provider(Some(json!({"name":"local"}))))["success"],
            true
        );
        assert_eq!(
            value(delete_provider(Some(json!({"name":"local"}))))["error"]["code"],
            "NOT_FOUND"
        );
        let parsed = value(parse_jd("Rust Engineer\n5 years SQL".into()));
        assert_eq!(parsed["success"], true);
        let jid = parsed["data"]["id"].as_str().unwrap();
        let matched = value(match_job(Some(json!({"jobDescId":jid,"personaId":"p1"}))));
        assert_eq!(matched["success"], true);
        let match_id = matched["data"]["id"].as_str().unwrap();
        let changed = value(update_match_status(Some(
            json!({"matchId":match_id,"status":"interested","expectedVersion":1}),
        )));
        assert_eq!(changed["success"], true);
        let stale = value(update_match_status(Some(
            json!({"matchId":match_id,"status":"applied","expectedVersion":1}),
        )));
        assert_eq!(stale["error"]["code"], "CONFLICT");
        let events = value(get_job_status_events(Some(json!({"matchId":match_id}))));
        assert_eq!(events["data"].as_array().unwrap().len(), 2);
        assert_eq!(events["data"][0]["fromStatus"], Value::Null);
        assert_eq!(events["data"][1]["fromStatus"], "new");
        assert_eq!(value(list_jobs(None))["success"], true);
        assert_eq!(
            value(get_job_matches(Some(json!({"jobDescId":jid}))))["success"],
            true
        );
        assert_eq!(
            value(get_skill_graph(None))["data"]
                .as_array()
                .unwrap()
                .len(),
            51
        );
        assert_eq!(
            value(search_skills(Some(json!({"query":"rust"}))))["success"],
            true
        );
        assert_eq!(value(get_learning_paths_by_source(None))["success"], true);
        assert_eq!(value(save_experience(None))["error"]["code"], "VALIDATION");
        assert_eq!(
            value(get_persona_by_id(Some(json!({"personaId":"missing"}))))["error"]["code"],
            "NOT_FOUND"
        );
        assert_eq!(
            value(update_match_status(Some(
                json!({"matchId":"missing","status":"applied","expectedVersion":1})
            )))["error"]["code"],
            "NOT_FOUND"
        );
        assert_eq!(
            value(create_custom_skill(Some(
                json!({"id":"cs1","ownerId":"default","name":"提示工程","category":"technical","level":1})
            )))["success"],
            true
        );
        assert_eq!(
            value(list_custom_skills(None))["data"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            value(update_custom_skill(Some(
                json!({"id":"cs1","ownerId":"default","name":"高级提示工程","category":"technical","level":2})
            )))["success"],
            true
        );
        assert_eq!(
            value(simulate_skill_what_if(Some(
                json!({"requiredSkills":["Rust","SQL"],"currentSkills":["Rust"],"hypotheticalSkills":["SQL"]})
            )))["data"]["delta"],
            50
        );
        {
            let c = connection().unwrap();
            c.execute("INSERT INTO learning_paths(id,persona_id,target_gap,status) VALUES('lp1','p1','Rust','active')",[]).unwrap();
            c.execute("INSERT INTO learning_items(id,path_id,skill_id,title,status) VALUES('li1','lp1','rust','实践','pending')",[]).unwrap();
        }
        assert_eq!(
            value(update_learning_progress(Some(
                json!({"pathId":"lp1","itemId":"li1","status":"in_progress","expectedVersion":1})
            )))["data"]["version"],
            2
        );
        assert_eq!(
            value(update_learning_progress(Some(
                json!({"pathId":"lp1","itemId":"li1","status":"completed","completionNote":"完成 CLI 项目","expectedVersion":2})
            )))["data"]["version"],
            3
        );
        let listed = value(get_learning_paths_by_source(Some(
            json!({"sourceType":"manual"}),
        )));
        assert_eq!(listed["data"][0]["items"][0]["status"], "completed");
        assert_eq!(listed["data"][0]["items"][0]["version"], 3);
        assert_eq!(
            listed["data"][0]["items"][0]["completionNote"],
            "完成 CLI 项目"
        );
        let stale = value(update_learning_progress(Some(
            json!({"pathId":"lp1","itemId":"li1","status":"skipped","expectedVersion":1}),
        )));
        assert_eq!(stale["error"]["code"], "CONFLICT");
        let invalid_conversion = value(complete_learning_to_experience(Some(
            json!({"itemId":"li1","experienceId":"bad","title":"","organization":"Self","rawDescription":"Body"}),
        )));
        assert_eq!(invalid_conversion["error"]["code"], "VALIDATION");
        {
            let c = connection().unwrap();
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM learning_conversions", [], |r| r
                    .get::<_, u32>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM experiences WHERE id='bad'", [], |r| r
                    .get::<_, u32>(0))
                    .unwrap(),
                0
            );
        }
        let converted = value(complete_learning_to_experience(Some(
            json!({"itemId":"li1","experienceId":"learn-exp","title":"Edited project","organization":"Self study","rawDescription":"Built an edited CLI"}),
        )));
        assert_eq!(converted["success"], true);
        assert_eq!(converted["data"]["draft"]["status"], "draft");
        assert_eq!(converted["data"]["draft"]["title"], "Edited project");
        {
            let c = connection().unwrap();
            let row:(String,String,String,String,i64)=c.query_row("SELECT title,organization,raw_description,status,version FROM experiences WHERE id='learn-exp'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
            assert_eq!(
                row,
                (
                    "Edited project".into(),
                    "Self study".into(),
                    "Built an edited CLI".into(),
                    "draft".into(),
                    1
                )
            );
            let revision: (i64, String) = c.query_row(
                "SELECT revision,source FROM experience_revisions WHERE experience_id='learn-exp'",
                [], |r| Ok((r.get(0)?, r.get(1)?)),
            ).unwrap();
            assert_eq!(revision, (1, "create".into()));
        }
        assert_eq!(
            value(complete_learning_to_experience(Some(
                json!({"itemId":"li1","experienceId":"learn-exp-2","title":"Again","organization":"Self","rawDescription":"Again"})
            )))["error"]["code"],
            "CONFLICT"
        );
        {
            let c = connection().unwrap();
            c.execute("DELETE FROM learning_paths WHERE id='lp1'", [])
                .unwrap();
            let after_path:(Option<String>,Option<String>,String)=c.query_row("SELECT item_id,experience_id,completion_note_snapshot FROM learning_conversions",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
            assert_eq!(
                after_path,
                (None, Some("learn-exp".into()), "完成 CLI 项目".into())
            );
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM experiences WHERE id='learn-exp'",
                    [],
                    |r| r.get::<_, u32>(0)
                )
                .unwrap(),
                1
            );
            c.execute("DELETE FROM experiences WHERE id='learn-exp'", [])
                .unwrap();
            let after_experience: (Option<String>, Option<String>, String) = c
                .query_row(
                    "SELECT item_id,experience_id,source_skill_id FROM learning_conversions",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert_eq!(after_experience, (None, None, "rust".into()));
        }
        assert_eq!(
            value(delete_custom_skill(Some(
                json!({"ownerId":"default","skillId":"cs1"})
            )))["success"],
            true
        );
        assert_eq!(
            value(get_persona_by_id(Some(json!({"personaId":"p1"}))))["success"],
            true
        );
        assert_eq!(
            value(update_persona(Some(
                json!({"personaId":"p1","data":{"name":"架构师","identityStatement":"可靠系统"}})
            )))["data"]["name"],
            "架构师"
        );
        let match_id = matched["data"]["id"].as_str().unwrap();
        assert_eq!(
            value(update_match_status(Some(
                json!({"matchId":match_id,"status":"applied","expectedVersion":2})
            )))["success"],
            true
        );
        {
            let c = connection().unwrap();
            c.execute("INSERT INTO job_match_experience_reframes(id,job_match_id,experience_id,original_summary,reframed_summary) VALUES('rf1',?1,'e1','original','suggested')",[match_id]).unwrap();
        }
        assert_eq!(
            value(get_reframe_results(Some(json!({"matchId":match_id}))))["data"]["count"],
            1
        );
        assert_eq!(
            value(update_reframe(Some(
                json!({"reframeId":"rf1","reframedSummary":"edited"})
            )))["success"],
            true
        );
        assert_eq!(
            value(reset_reframe(Some(json!({"reframeId":"rf1"}))))["success"],
            true
        );
        assert_eq!(
            value(get_skill_resources(Some(
                json!({"skillId":"user_research"})
            )))["success"],
            true
        );
        assert_eq!(
            value(delete_job(Some(json!({"jobDescId":jid}))))["success"],
            true
        );
        assert_eq!(
            value(delete_experience(Some(
                json!({"experienceId":"e1","version":1})
            )))["success"],
            true
        );
        assert_eq!(
            value(delete_persona(Some(json!({"personaId":"p1"}))))["success"],
            true
        );
        std::env::remove_var("CAREERCRAFT_DB_PATH");
    }
    #[test]
    fn runtime_contract_vectors_reject_invalid_payloads_without_side_effects() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vectors.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let reads: [CommandCase; 7] = [
            ("getExperiences", get_experiences),
            ("getPersonas", get_personas),
            ("listJobs", list_jobs),
            ("getLearningPathsBySource", get_learning_paths_by_source),
            ("getSkillGraph", get_skill_graph),
            ("getSettings", get_settings),
            ("listCustomSkills", list_custom_skills),
        ];
        for (name, call) in reads {
            let v = value(call(None));
            assert_eq!(v["success"], true, "{name} must return a success envelope")
        }
        let invalid: [CommandCase; 43] = [
            ("saveExperience", save_experience),
            ("deleteExperience", delete_experience),
            ("getPersonaById", get_persona_by_id),
            ("createPersona", create_persona),
            ("updatePersona", update_persona),
            ("deletePersona", delete_persona),
            ("getExperiencesWithFitScore", get_experiences_with_fit_score),
            ("updateFitScore", update_fit_score),
            ("generateResume", generate_resume),
            ("previewResume", preview_resume),
            ("checkUpdate", check_update),
            ("downloadUpdate", download_update),
            ("installUpdate", install_update),
            ("exportResumePDF", export_resume_pdf),
            ("chatRefineResume", chat_refine_resume),
            ("saveSettings", save_settings),
            ("testLLMConnection", test_llm_connection),
            ("importExperiences", import_experiences),
            ("importFile", import_file),
            ("matchJob", match_job),
            ("deleteJob", delete_job),
            ("getJobMatches", get_job_matches),
            ("getJobStatusEvents", get_job_status_events),
            ("updateMatchStatus", update_match_status),
            ("reframeResume", reframe_resume),
            ("getReframeResults", get_reframe_results),
            ("updateReframe", update_reframe),
            ("resetReframe", reset_reframe),
            ("getLearningPath", get_learning_path),
            ("getSkillResources", get_skill_resources),
            ("searchSkills", search_skills),
            ("createCustomSkill", create_custom_skill),
            ("updateCustomSkill", update_custom_skill),
            ("deleteCustomSkill", delete_custom_skill),
            ("simulateSkillWhatIf", simulate_skill_what_if),
            ("updateLearningProgress", update_learning_progress),
            (
                "completeLearningToExperience",
                complete_learning_to_experience,
            ),
            ("resetFitScore", reset_fit_score),
            ("deleteProvider", delete_provider),
            ("openExternalUrl", open_external_url),
            ("collectJobUrl", collect_job_url),
            ("listResumeVersions", list_resume_versions),
            ("restoreResumeVersion", restore_resume_version),
        ];
        for (name, call) in invalid {
            let v = value(call(None));
            assert_eq!(v["success"], false, "{name} accepted an invalid payload");
            let code = v["error"]["code"].as_str().unwrap_or("");
            assert!(
                ["VALIDATION", "NOT_FOUND", "CONFLICT"].contains(&code)
                    || (["checkUpdate", "downloadUpdate", "installUpdate"].contains(&name)
                        && code == "UNAVAILABLE"),
                "{name} returned forbidden code {code}"
            )
        }
        let parsed = value(parse_jd(" ".into()));
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "VALIDATION");
        let diff = value(diff_resume_versions(None));
        assert_eq!(diff["success"], false);
        assert_ne!(diff["error"]["code"], "UNAVAILABLE");
        std::env::remove_var("CAREERCRAFT_DB_PATH");
    }
    #[test]
    fn experience_command_closes_lifecycle_and_returns_overlap_warning() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experience-state.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let first = value(save_experience(Some(
            json!({"newId":"first","type":"work","title":"First","startDate":"2024-01","endDate":"2024-12"}),
        )));
        assert_eq!(first["data"]["status"], "draft");
        let second = value(save_experience(Some(
            json!({"newId":"second","type":"project","title":"Second","startDate":"2024-06","endDate":"2025-01"}),
        )));
        assert_eq!(second["data"]["warnings"][0]["code"], "DATE_OVERLAP");
        assert_eq!(second["data"]["overlapExperienceIds"], json!(["first"]));
        let confirmed = value(save_experience(Some(
            json!({"id":"first","version":1,"status":"confirmed"}),
        )));
        assert_eq!(confirmed["data"]["status"], "confirmed");
        let regression = value(save_experience(Some(
            json!({"id":"first","version":2,"status":"draft"}),
        )));
        assert_eq!(regression["error"]["code"], "CONFLICT");
        let discarded = value(save_experience(Some(
            json!({"id":"second","version":1,"status":"discarded"}),
        )));
        assert_eq!(discarded["data"]["status"], "discarded");
        std::env::remove_var("CAREERCRAFT_DB_PATH");
    }
    #[test]
    fn fallback_cache_never_preempts_persona_preferred_route() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("route-cache.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let c = Connection::open(&path).unwrap();
        c.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences,preferred_model) VALUES('p','u','P',1,'{}','[]',5,'preferred/m1')",[]).unwrap();
        for (name, model) in [("fallback", "m2"), ("preferred", "m1")] {
            c.execute("INSERT INTO provider_configs(name,base_url,default_model,credential_target,enabled) VALUES(?1,'https://example.com',?2,'ref',1)",rusqlite::params![name,model]).unwrap();
        }
        let routes = ordered_model_routes(&c, Some("p")).unwrap();
        assert_eq!(routes[0], ("preferred".into(), "m1".into()));
        drop(c);
        let request = crate::domain::llm::GenerationRequest {
            messages: vec![crate::domain::llm::LlmMessage {
                role: crate::domain::llm::LlmRole::User,
                content: "prompt".into(),
            }],
            preferred: None,
            temperature: 0.2,
            max_output_tokens: 10,
        };
        let fallback_key = crate::infra::llm_cache::key("op", "v", "fallback", "m2", &request);
        crate::infra::llm_cache::put_success(
            &fallback_key,
            "op",
            "v",
            &crate::domain::llm::GenerationResult {
                text: "stale fallback".into(),
                provider: "fallback".into(),
                model: "m2".into(),
            },
            100,
        )
        .unwrap();
        assert_eq!(
            first_route_cache(&routes, &request, "op", "v", 101).unwrap(),
            None
        );
        std::env::remove_var("CAREERCRAFT_DB_PATH")
    }
}
