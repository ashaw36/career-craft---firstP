//! Provider orchestration with streaming, fallback, retry and cancellation (CC-FR-008/021).
//! Secrets are referenced by provider name only and must be resolved by the host's
//! credential-store adapter (CC-SEC-001/002); this layer never receives persisted keys.
use crate::domain::llm::{
    GenerationRequest, GenerationResult, LlmError, LlmErrorKind, ModelRef, StreamEvent,
};

pub trait Cancellation {
    fn is_cancelled(&self) -> bool;
}

pub trait EventSink {
    fn emit(&mut self, event: StreamEvent);
}

pub trait LlmProvider {
    fn name(&self) -> &str;
    fn generate(
        &self,
        model: &str,
        request: &GenerationRequest,
        cancel: &dyn Cancellation,
        sink: &mut dyn EventSink,
    ) -> Result<String, LlmError>;
}

pub trait RetrySleeper {
    fn sleep_ms(&self, delay_ms: u64);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts_per_provider: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts_per_provider: 2,
            initial_delay_ms: 300,
            max_delay_ms: 2_000,
        }
    }
}

impl RetryPolicy {
    pub fn delay_for(&self, completed_attempts: u32) -> u64 {
        self.initial_delay_ms
            .saturating_mul(1_u64 << completed_attempts.min(20))
            .min(self.max_delay_ms)
    }
}

/// `routes` should put `Persona.preferred_model` first, then configured fallbacks.
pub fn generate_with_fallback(
    providers: &[&dyn LlmProvider],
    routes: &[ModelRef],
    request: &GenerationRequest,
    policy: &RetryPolicy,
    sleeper: &dyn RetrySleeper,
    cancel: &dyn Cancellation,
    sink: &mut dyn EventSink,
) -> Result<GenerationResult, LlmError> {
    if routes.is_empty() || request.messages.is_empty() {
        return Err(LlmError {
            kind: LlmErrorKind::InvalidRequest,
            message: "at least one route and message are required".into(),
        });
    }
    let mut last_error = None;
    for route in routes {
        let Some(provider) = providers.iter().find(|p| p.name() == route.provider) else {
            continue;
        };
        for attempt in 1..=policy.max_attempts_per_provider.max(1) {
            if cancel.is_cancelled() {
                sink.emit(StreamEvent::Cancelled);
                return Err(LlmError {
                    kind: LlmErrorKind::Cancelled,
                    message: "generation cancelled".into(),
                });
            }
            sink.emit(StreamEvent::Started {
                provider: route.provider.clone(),
                model: route.model.clone(),
                attempt,
            });
            match provider.generate(&route.model, request, cancel, sink) {
                Ok(text) => {
                    sink.emit(StreamEvent::Completed {
                        finish_reason: Some("stop".into()),
                    });
                    return Ok(GenerationResult {
                        text,
                        provider: route.provider.clone(),
                        model: route.model.clone(),
                    });
                }
                Err(error) => {
                    let retryable = error.retryable();
                    sink.emit(StreamEvent::Failed {
                        code: format!("{:?}", error.kind),
                        retryable,
                    });
                    last_error = Some(error);
                    if !retryable {
                        break;
                    }
                    if attempt < policy.max_attempts_per_provider {
                        let delay_ms = policy.delay_for(attempt - 1);
                        sink.emit(StreamEvent::Retrying {
                            provider: route.provider.clone(),
                            attempt: attempt + 1,
                            delay_ms,
                        });
                        sleeper.sleep_ms(delay_ms);
                    }
                }
            }
        }
    }
    Err(last_error.unwrap_or(LlmError {
        kind: LlmErrorKind::Provider,
        message: "no configured provider matched the routes".into(),
    }))
}

pub fn preferred_routes(preferred: Option<ModelRef>, configured: &[ModelRef]) -> Vec<ModelRef> {
    let mut routes = Vec::new();
    if let Some(value) = preferred {
        routes.push(value);
    }
    for value in configured {
        if !routes.contains(value) {
            routes.push(value.clone());
        }
    }
    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::{LlmMessage, LlmRole};
    use std::cell::Cell;

    struct NeverCancel;
    impl Cancellation for NeverCancel {
        fn is_cancelled(&self) -> bool {
            false
        }
    }
    struct AlwaysCancel;
    impl Cancellation for AlwaysCancel {
        fn is_cancelled(&self) -> bool {
            true
        }
    }
    struct NoSleep;
    impl RetrySleeper for NoSleep {
        fn sleep_ms(&self, _: u64) {}
    }
    #[derive(Default)]
    struct Events(Vec<StreamEvent>);
    impl EventSink for Events {
        fn emit(&mut self, event: StreamEvent) {
            self.0.push(event);
        }
    }
    struct Fake {
        name: &'static str,
        calls: Cell<u32>,
        fail_until: u32,
    }
    impl LlmProvider for Fake {
        fn name(&self) -> &str {
            self.name
        }
        fn generate(
            &self,
            _: &str,
            _: &GenerationRequest,
            _: &dyn Cancellation,
            sink: &mut dyn EventSink,
        ) -> Result<String, LlmError> {
            let call = self.calls.get() + 1;
            self.calls.set(call);
            if call <= self.fail_until {
                return Err(LlmError {
                    kind: LlmErrorKind::Timeout,
                    message: "timeout".into(),
                });
            }
            sink.emit(StreamEvent::Delta { text: "ok".into() });
            Ok("ok".into())
        }
    }
    fn request() -> GenerationRequest {
        GenerationRequest {
            messages: vec![LlmMessage {
                role: LlmRole::User,
                content: "hello".into(),
            }],
            preferred: None,
            temperature: 0.2,
            max_output_tokens: 100,
        }
    }

    #[test]
    fn retries_then_falls_back() {
        let first = Fake {
            name: "a",
            calls: Cell::new(0),
            fail_until: 9,
        };
        let second = Fake {
            name: "b",
            calls: Cell::new(0),
            fail_until: 0,
        };
        let routes = vec![
            ModelRef {
                provider: "a".into(),
                model: "one".into(),
            },
            ModelRef {
                provider: "b".into(),
                model: "two".into(),
            },
        ];
        let mut events = Events::default();
        let result = generate_with_fallback(
            &[&first, &second],
            &routes,
            &request(),
            &RetryPolicy::default(),
            &NoSleep,
            &NeverCancel,
            &mut events,
        )
        .unwrap();
        assert_eq!(first.calls.get(), 2);
        assert_eq!(result.provider, "b");
        assert!(events
            .0
            .iter()
            .any(|e| matches!(e, StreamEvent::Retrying { .. })));
    }

    #[test]
    fn persona_preference_is_first_without_duplicate() {
        let preferred = ModelRef {
            provider: "openai".into(),
            model: "preferred".into(),
        };
        let routes = preferred_routes(
            Some(preferred.clone()),
            &[
                preferred.clone(),
                ModelRef {
                    provider: "local".into(),
                    model: "fallback".into(),
                },
            ],
        );
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0], preferred);
    }

    #[test]
    fn cancellation_stops_before_provider_call() {
        let provider = Fake {
            name: "a",
            calls: Cell::new(0),
            fail_until: 0,
        };
        let routes = vec![ModelRef {
            provider: "a".into(),
            model: "one".into(),
        }];
        let mut events = Events::default();
        let error = generate_with_fallback(
            &[&provider],
            &routes,
            &request(),
            &RetryPolicy::default(),
            &NoSleep,
            &AlwaysCancel,
            &mut events,
        )
        .unwrap_err();
        assert_eq!(error.kind, LlmErrorKind::Cancelled);
        assert_eq!(provider.calls.get(), 0);
        assert!(matches!(events.0.last(), Some(StreamEvent::Cancelled)));
    }
}
