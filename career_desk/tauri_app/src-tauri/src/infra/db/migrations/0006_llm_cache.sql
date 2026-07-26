CREATE TABLE IF NOT EXISTS llm_cache (
 cache_key TEXT PRIMARY KEY,
 operation TEXT NOT NULL,
 prompt_version TEXT NOT NULL,
 provider TEXT NOT NULL,
 model TEXT NOT NULL,
 response_text TEXT NOT NULL,
 created_at INTEGER NOT NULL,
 expires_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_llm_cache_expires ON llm_cache(expires_at);
