//! Central security-safe diagnostic helpers (CC-NFR-005/CC-SEC-001/006).
pub fn redact_diagnostic(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| {
            let lower = token.to_ascii_lowercase();
            if lower.starts_with("bearer")
                || lower.starts_with("sk-")
                || lower.contains("apikey=")
                || lower.contains("api_key=")
                || lower.contains("password=")
                || lower.contains("token=")
            {
                "[REDACTED]".to_owned()
            } else {
                token.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_common_secret_shapes_without_echo() {
        let value = redact_diagnostic("failed bearer sk-secret apiKey=abc password=p token=t safe");
        assert!(
            !value.contains("secret")
                && !value.contains("abc")
                && !value.contains("password=p")
                && !value.contains("token=t")
        );
        assert!(value.contains("safe"))
    }
}
