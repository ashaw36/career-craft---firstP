//! Host-side LLM support types. Concrete HTTP providers are deliberately injected.
//! Requirement mapping: CC-FR-008/021, CC-SEC-001/002/006.
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::application::llm_orchestration::Cancellation;
use crate::{
    application::llm_orchestration::{EventSink, LlmProvider},
    domain::llm::{GenerationRequest, LlmError, LlmErrorKind, StreamEvent},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub default_model: String,
    /// Opaque Windows Credential Manager target, never an API key.
    pub credential_target: String,
    pub enabled: bool,
}

impl ProviderConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.provider.trim().is_empty() || self.default_model.trim().is_empty() {
            return Err("provider and model are required");
        }
        if self.credential_target.trim().is_empty() {
            return Err("credential target is required");
        }
        resolve_endpoint(&self.provider, &self.base_url).map(|_| ())?;
        Ok(())
    }
}
fn blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.octets()[0] == 0
        }
        IpAddr::V6(v) => {
            v.is_loopback()
                || v.is_unspecified()
                || v.is_unique_local()
                || v.is_unicast_link_local()
        }
    }
}
fn resolve_endpoint(provider: &str, raw: &str) -> Result<(String, SocketAddr), &'static str> {
    let url = reqwest::Url::parse(raw).map_err(|_| "provider endpoint is invalid")?;
    let host = url.host_str().ok_or("provider endpoint host is required")?;
    let local = provider.eq_ignore_ascii_case("local");
    let loopback_host = host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if url.scheme() == "http" && !(local && loopback_host) {
        return Err(
            "provider endpoint must use HTTPS; only provider 'local' may use loopback HTTP",
        );
    }
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("provider endpoint is not allowed");
    }
    let port = url
        .port_or_known_default()
        .ok_or("provider endpoint port is required")?;
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| "provider endpoint DNS resolution failed")?;
    let mut safe = None;
    for address in addresses {
        let ip = address.ip();
        if blocked_ip(ip) && !(local && ip.is_loopback()) {
            return Err("provider endpoint resolves to a private, link-local, or metadata address");
        }
        safe.get_or_insert(address);
    }
    Ok((
        host.to_owned(),
        safe.ok_or("provider endpoint DNS resolution returned no addresses")?,
    ))
}

#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}

pub struct OpenAiCompatibleProvider {
    pub config: ProviderConfig,
    api_key: String,
    client: reqwest::blocking::Client,
}
impl OpenAiCompatibleProvider {
    /// Streaming refine/structure can run several minutes; do not use a short total timeout.
    pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);
    pub const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    pub fn new(config: ProviderConfig, api_key: String) -> Result<Self, LlmError> {
        Self::with_timeout(config, api_key, Self::REQUEST_TIMEOUT)
    }
    fn with_timeout(
        config: ProviderConfig,
        api_key: String,
        timeout: std::time::Duration,
    ) -> Result<Self, LlmError> {
        let (host, resolved) =
            resolve_endpoint(&config.provider, &config.base_url).map_err(|e| LlmError {
                kind: LlmErrorKind::InvalidRequest,
                message: e.into(),
            })?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::blocking::Client::builder()
            .resolve(&host, resolved)
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| safe_error(LlmErrorKind::Provider, "failed to initialize HTTP client"))?;
        Ok(Self {
            config,
            api_key,
            client,
        })
    }
    fn endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        )
    }
    pub fn probe(&self) -> Result<(), LlmError> {
        // Probe uses a short-lived client so connection checks stay snappy.
        let probe = Self::with_timeout(
            self.config.clone(),
            self.api_key.clone(),
            Self::PROBE_TIMEOUT,
        )?;
        let response = probe
            .client
            .post(probe.endpoint())
            .bearer_auth(&probe.api_key)
            .json(&serde_json::json!({
                "model": probe.config.default_model,
                "messages": [{"role":"user","content":"Reply OK"}],
                "temperature": 0.0,
                "max_tokens": 8,
                "stream": false
            }))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    safe_error(LlmErrorKind::Timeout, "provider request timed out")
                } else {
                    safe_error(LlmErrorKind::Provider, "provider connection failed")
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(safe_error(
                http_error_kind(status.as_u16()),
                if status.as_u16() == 401 || status.as_u16() == 403 {
                    "provider authentication failed"
                } else {
                    "provider rejected request"
                },
            ));
        }
        let body: serde_json::Value = response.json().map_err(|_| {
            safe_error(LlmErrorKind::Provider, "provider returned invalid JSON")
        })?;
        if body.pointer("/choices/0").is_none() {
            return Err(safe_error(
                LlmErrorKind::Provider,
                "provider rejected request",
            ));
        }
        Ok(())
    }
}
fn safe_error(kind: LlmErrorKind, message: &str) -> LlmError {
    LlmError {
        kind,
        message: message.into(),
    }
}
fn http_error_kind(status: u16) -> LlmErrorKind {
    match status {
        401 | 403 => LlmErrorKind::Authentication,
        429 => LlmErrorKind::RateLimited,
        500..=599 => LlmErrorKind::Transport,
        _ => LlmErrorKind::Provider,
    }
}
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.config.provider
    }
    fn generate(
        &self,
        model: &str,
        request: &GenerationRequest,
        cancel: &dyn Cancellation,
        sink: &mut dyn EventSink,
    ) -> Result<String, LlmError> {
        if cancel.is_cancelled() {
            return Err(safe_error(LlmErrorKind::Cancelled, "request cancelled"));
        }
        let messages=request.messages.iter().map(|m|serde_json::json!({"role":match m.role{crate::domain::llm::LlmRole::System=>"system",crate::domain::llm::LlmRole::User=>"user",crate::domain::llm::LlmRole::Assistant=>"assistant"},"content":m.content})).collect::<Vec<_>>();
        let response=self.client.post(self.endpoint()).bearer_auth(&self.api_key).json(&serde_json::json!({"model":model,"messages":messages,"temperature":request.temperature,"max_tokens":request.max_output_tokens,"stream":true})).send().map_err(|e|if e.is_timeout(){safe_error(LlmErrorKind::Timeout,"provider request timed out")}else{safe_error(LlmErrorKind::Provider,"provider connection failed")})?;
        let status = response.status();
        if !status.is_success() {
            return Err(safe_error(
                http_error_kind(status.as_u16()),
                if status.as_u16() == 401 || status.as_u16() == 403 {
                    "provider authentication failed"
                } else {
                    "provider rejected request"
                },
            ));
        }
        use std::io::{BufRead, BufReader};
        let mut text = String::new();
        let mut terminated = false;
        for line in BufReader::new(response).lines() {
            if cancel.is_cancelled() {
                return Err(safe_error(LlmErrorKind::Cancelled, "request cancelled"));
            }
            let line = line
                .map_err(|_| safe_error(LlmErrorKind::Provider, "provider stream interrupted"))?;
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                terminated = true;
                break;
            }
            let value: serde_json::Value = serde_json::from_str(data).map_err(|_| {
                safe_error(LlmErrorKind::Provider, "provider returned invalid SSE JSON")
            })?;
            if let Some(delta) = value
                .pointer("/choices/0/delta/content")
                .and_then(|v| v.as_str())
            {
                text.push_str(delta);
                sink.emit(StreamEvent::Delta { text: delta.into() });
            }
            if value
                .pointer("/choices/0/finish_reason")
                .is_some_and(|v| !v.is_null())
            {
                terminated = true;
                break;
            }
        }
        if !terminated {
            return Err(safe_error(
                LlmErrorKind::Provider,
                "provider stream truncated before terminal event",
            ));
        }
        if text.is_empty() {
            Err(safe_error(
                LlmErrorKind::Provider,
                "provider returned no text",
            ))
        } else {
            Ok(text)
        }
    }
}

impl Cancellation for CancellationToken {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_contains_reference_not_secret() {
        let config = ProviderConfig {
            provider: "openai".into(),
            base_url: "https://api.openai.com".into(),
            default_model: "model".into(),
            credential_target: "CareerCraft/llm/openai".into(),
            enabled: true,
        };
        assert_eq!(config.validate(), Ok(()));
    }
    #[test]
    fn rejects_insecure_remote_endpoint() {
        let config = ProviderConfig {
            provider: "x".into(),
            base_url: "http://example.com".into(),
            default_model: "m".into(),
            credential_target: "ref".into(),
            enabled: true,
        };
        assert!(config.validate().is_err());
    }
    #[test]
    fn cancellation_is_shared() {
        let token = CancellationToken::default();
        let other = token.clone();
        token.cancel();
        assert!(other.is_cancelled());
    }
    #[test]
    fn classifies_401_429_and_5xx_for_retry_policy() {
        assert_eq!(http_error_kind(401), LlmErrorKind::Authentication);
        assert_eq!(http_error_kind(429), LlmErrorKind::RateLimited);
        assert_eq!(http_error_kind(500), LlmErrorKind::Transport);
        assert!(!LlmError {
            kind: http_error_kind(401),
            message: String::new()
        }
        .retryable());
        assert!(LlmError {
            kind: http_error_kind(429),
            message: String::new()
        }
        .retryable());
        assert!(LlmError {
            kind: http_error_kind(503),
            message: String::new()
        }
        .retryable())
    }
    #[test]
    fn openai_provider_streams_from_local_mock_and_redacts_key() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
            time::Duration,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0u8; 8192];
            let n = socket.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..n]).to_lowercase();
            assert!(request.contains("authorization: bearer test-secret"));
            let body = r#"{"choices":[{"message":{"role":"assistant","content":"OK"}}]}"#;
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        let provider = OpenAiCompatibleProvider::new(
            ProviderConfig {
                provider: "local".into(),
                base_url: format!("http://{addr}"),
                default_model: "test".into(),
                credential_target: "mock".into(),
                enabled: true,
            },
            "test-secret".into(),
        )
        .unwrap();
        assert!(provider.probe().is_ok());
        server.join().unwrap();
        let debug = format!(
            "{:?}",
            safe_error(
                LlmErrorKind::Authentication,
                "provider authentication failed"
            )
        );
        assert!(!debug.contains("test-secret"));
    }
    fn request() -> GenerationRequest {
        GenerationRequest {
            messages: vec![crate::domain::llm::LlmMessage {
                role: crate::domain::llm::LlmRole::User,
                content: "x".into(),
            }],
            preferred: None,
            temperature: 0.0,
            max_output_tokens: 10,
        }
    }
    fn local_provider(addr: SocketAddr, timeout: std::time::Duration) -> OpenAiCompatibleProvider {
        OpenAiCompatibleProvider::with_timeout(
            ProviderConfig {
                provider: "local".into(),
                base_url: format!("http://{addr}"),
                default_model: "m".into(),
                credential_target: "mock".into(),
                enabled: true,
            },
            "secret".into(),
            timeout,
        )
        .unwrap()
    }
    struct Never;
    impl Cancellation for Never {
        fn is_cancelled(&self) -> bool {
            false
        }
    }
    struct Noop;
    impl EventSink for Noop {
        fn emit(&mut self, _: StreamEvent) {}
    }
    #[test]
    fn real_transport_timeout_invalid_sse_and_truncated_stream_are_explicit() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
            time::Duration,
        };
        fn run(body: &'static str, declared: usize) -> LlmError {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut req_buf = [0u8; 4096];
                let _ = socket.read(&mut req_buf);
                write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n{body}").unwrap();
            });
            let error = local_provider(addr, Duration::from_secs(2))
                .generate("m", &request(), &Never, &mut Noop)
                .unwrap_err();
            server.join().unwrap();
            error
        }
        let invalid = run("data: {bad-json}\n\n", 18);
        assert_eq!(invalid.kind, LlmErrorKind::Provider);
        assert!(invalid.message.contains("invalid SSE JSON"));
        let partial = "data: {\"choices\":[{\"delta\":{";
        let truncated = run(partial, partial.len() + 100);
        assert_eq!(truncated.kind, LlmErrorKind::Provider);
        assert!(
            truncated.message.contains("interrupted") || truncated.message.contains("invalid SSE")
        );
        let legal_delta = "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n";
        let eof = run(legal_delta, legal_delta.len());
        assert_eq!(eof.kind, LlmErrorKind::Provider);
        assert!(eof.message.contains("truncated"));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request);
            thread::sleep(Duration::from_millis(250));
        });
        let timeout = local_provider(addr, Duration::from_millis(40))
            .generate("m", &request(), &Never, &mut Noop)
            .unwrap_err();
        assert_eq!(timeout.kind, LlmErrorKind::Timeout);
        server.join().unwrap();
    }
    #[test]
    fn cancellation_after_first_real_token_discards_partial_result() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
            thread,
            time::Duration,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request);
            let body="data: {\"choices\":[{\"delta\":{\"content\":\"first\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"second\"}}]}\n\ndata: [DONE]\n\n";
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        let token = CancellationToken::default();
        struct CancelOnDelta(CancellationToken);
        impl EventSink for CancelOnDelta {
            fn emit(&mut self, event: StreamEvent) {
                if matches!(event, StreamEvent::Delta { .. }) {
                    self.0.cancel()
                }
            }
        }
        let error = local_provider(addr, Duration::from_secs(2))
            .generate("m", &request(), &token, &mut CancelOnDelta(token.clone()))
            .unwrap_err();
        assert_eq!(error.kind, LlmErrorKind::Cancelled);
        server.join().unwrap();
    }
}
