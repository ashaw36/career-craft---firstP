//! LLM domain contracts (CC-FR-008/021, CC-SEC-001/002/006).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationRequest {
    pub messages: Vec<LlmMessage>,
    pub preferred: Option<ModelRef>,
    pub temperature: f32,
    pub max_output_tokens: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEvent {
    Started {
        provider: String,
        model: String,
        attempt: u32,
    },
    Delta {
        text: String,
    },
    Completed {
        finish_reason: Option<String>,
    },
    Retrying {
        provider: String,
        attempt: u32,
        delay_ms: u64,
    },
    Failed {
        code: String,
        retryable: bool,
    },
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationResult {
    pub text: String,
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlmErrorKind {
    InvalidRequest,
    Authentication,
    RateLimited,
    Timeout,
    Transport,
    Provider,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmError {
    pub kind: LlmErrorKind,
    pub message: String,
}

impl LlmError {
    pub fn retryable(&self) -> bool {
        matches!(
            self.kind,
            LlmErrorKind::RateLimited | LlmErrorKind::Timeout | LlmErrorKind::Transport
        )
    }
}
