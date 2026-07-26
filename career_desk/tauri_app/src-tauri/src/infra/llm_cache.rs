use crate::{
    domain::llm::{GenerationRequest, GenerationResult},
    error::AppError,
};
use rusqlite::params;
use std::sync::{Mutex, MutexGuard, OnceLock};
pub const TTL_SECONDS: i64 = 7 * 24 * 60 * 60;
static SINGLE_FLIGHT: OnceLock<Mutex<()>> = OnceLock::new();
pub fn single_flight() -> MutexGuard<'static, ()> {
    SINGLE_FLIGHT
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}
fn hash(bytes: &[u8]) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3)
    }
    format!("{h:016x}")
}
pub fn key(
    operation: &str,
    prompt_version: &str,
    provider: &str,
    model: &str,
    request: &GenerationRequest,
) -> String {
    let mut input = format!(
        "{operation}\0{prompt_version}\0{provider}\0{model}\0{}\0{}",
        request.temperature.to_bits(),
        request.max_output_tokens
    );
    for message in &request.messages {
        input.push('\0');
        input.push_str(match message.role {
            crate::domain::llm::LlmRole::System => "system",
            crate::domain::llm::LlmRole::User => "user",
            crate::domain::llm::LlmRole::Assistant => "assistant",
        });
        input.push('\0');
        input.push_str(&message.content)
    }
    hash(input.as_bytes())
}
pub fn get(cache_key: &str, now: i64) -> Result<Option<GenerationResult>, AppError> {
    let c = crate::infra::db::open_runtime_connection()?;
    let mut s = c.prepare(
        "SELECT response_text,provider,model FROM llm_cache WHERE cache_key=?1 AND expires_at>?2",
    )?;
    let mut rows = s.query(params![cache_key, now])?;
    if let Some(row) = rows.next()? {
        Ok(Some(GenerationResult {
            text: row.get(0)?,
            provider: row.get(1)?,
            model: row.get(2)?,
        }))
    } else {
        Ok(None)
    }
}
pub fn put_success(
    cache_key: &str,
    operation: &str,
    prompt_version: &str,
    result: &GenerationResult,
    now: i64,
) -> Result<(), AppError> {
    let c = crate::infra::db::open_runtime_connection()?;
    c.execute("INSERT INTO llm_cache(cache_key,operation,prompt_version,provider,model,response_text,created_at,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(cache_key) DO NOTHING",params![cache_key,operation,prompt_version,result.provider,result.model,result.text,now,now+TTL_SECONDS])?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::{GenerationRequest, LlmMessage, LlmRole};
    fn request(text: &str) -> GenerationRequest {
        GenerationRequest {
            messages: vec![LlmMessage {
                role: LlmRole::User,
                content: text.into(),
            }],
            preferred: None,
            temperature: 0.2,
            max_output_tokens: 10,
        }
    }
    #[test]
    fn key_covers_all_dimensions_without_storing_prompt_or_secret() {
        let base = key("op", "v1", "provider", "model", &request("secret prompt"));
        for other in [
            key("op2", "v1", "provider", "model", &request("secret prompt")),
            key("op", "v2", "provider", "model", &request("secret prompt")),
            key("op", "v1", "p2", "model", &request("secret prompt")),
            key("op", "v1", "provider", "m2", &request("secret prompt")),
            key("op", "v1", "provider", "model", &request("different")),
        ] {
            assert_ne!(base, other)
        }
        assert!(!base.contains("secret"))
    }
    #[test]
    fn persists_success_with_ttl_unique_key_and_no_prompt_column() {
        let _guard = crate::interface::commands::tests::ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        drop(crate::infra::db::Database::open_and_migrate(&path).unwrap());
        std::env::set_var("CAREERCRAFT_DB_PATH", &path);
        let result = GenerationResult {
            text: "ok".into(),
            provider: "p".into(),
            model: "m".into(),
        };
        put_success("key", "op", "v", &result, 100).unwrap();
        put_success("key", "op", "v", &result, 100).unwrap();
        assert_eq!(get("key", 101).unwrap(), Some(result));
        assert_eq!(get("key", 100 + TTL_SECONDS).unwrap(), None);
        let c = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM llm_cache", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        let columns = c
            .prepare("SELECT name FROM pragma_table_info('llm_cache')")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(!columns
            .iter()
            .any(|v| v.contains("prompt") && v != "prompt_version"));
        std::env::remove_var("CAREERCRAFT_DB_PATH")
    }
}
