//! Safe URL collection request/result model (CC-FR-020, CC-SEC-004/006).
//! The concrete `reqwest` adapter belongs in platform wiring; this module guarantees
//! that failure always returns an actionable manual-input path.
use std::{
    io::Read,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlCollectionRequest {
    pub url: String,
    pub host: String,
    pub resolved_addr: SocketAddr,
    pub max_bytes: usize,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlRejection {
    UnsupportedScheme,
    CredentialsInUrl,
    LocalFile,
    PrivateNetwork,
    InvalidUrl,
}

impl UrlCollectionRequest {
    pub fn new(url: &str) -> Result<Self, UrlRejection> {
        let value = url.trim();
        if value.starts_with("file:") {
            return Err(UrlRejection::LocalFile);
        }
        if !(value.starts_with("https://") || value.starts_with("http://")) {
            return Err(UrlRejection::UnsupportedScheme);
        }
        let parsed = reqwest::Url::parse(value).map_err(|_| UrlRejection::InvalidUrl)?;
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(UrlRejection::CredentialsInUrl);
        }
        let host = parsed
            .host_str()
            .ok_or(UrlRejection::InvalidUrl)?
            .to_lowercase();
        if is_obviously_private(&host) {
            return Err(UrlRejection::PrivateNetwork);
        }
        let port = parsed
            .port_or_known_default()
            .ok_or(UrlRejection::InvalidUrl)?;
        let addresses = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|_| UrlRejection::InvalidUrl)?;
        let mut safe = None;
        for address in addresses {
            if private_ip(address.ip()) {
                return Err(UrlRejection::PrivateNetwork);
            }
            safe.get_or_insert(address);
        }
        let resolved_addr = safe.ok_or(UrlRejection::InvalidUrl)?;
        Ok(Self {
            url: value.to_owned(),
            host,
            resolved_addr,
            max_bytes: 2 * 1024 * 1024,
            timeout_ms: 15_000,
        })
    }
}
fn private_ip(ip: IpAddr) -> bool {
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

fn is_obviously_private(host: &str) -> bool {
    if matches!(host, "localhost" | "::1" | "0.0.0.0")
        || host.ends_with(".localhost")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
    {
        return true;
    }
    let mut parts = host.split('.');
    matches!((parts.next(), parts.next()), (Some("172"), Some(second)) if second.parse::<u8>().is_ok_and(|n| (16..=31).contains(&n)))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionOutcome {
    Collected {
        final_url: String,
        text: String,
    },
    ManualInputRequired {
        original_url: String,
        reason: ManualFallbackReason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManualFallbackReason {
    LoginRequired,
    RobotsOrAccessDenied,
    UnsupportedPage,
    Network,
    EmptyContent,
    SizeLimit,
}

impl CollectionOutcome {
    pub fn fallback(request: &UrlCollectionRequest, reason: ManualFallbackReason) -> Self {
        Self::ManualInputRequired {
            original_url: request.url.clone(),
            reason,
        }
    }
}

pub fn collect(url: &str) -> CollectionOutcome {
    let request = match UrlCollectionRequest::new(url) {
        Ok(v) => v,
        Err(_) => {
            return CollectionOutcome::ManualInputRequired {
                original_url: url.into(),
                reason: ManualFallbackReason::UnsupportedPage,
            }
        }
    };
    let client = match reqwest::blocking::Client::builder()
        .resolve(&request.host, request.resolved_addr)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_millis(request.timeout_ms))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(v) => v,
        Err(_) => return CollectionOutcome::fallback(&request, ManualFallbackReason::Network),
    };
    let response = match client.get(&request.url).send() {
        Ok(v) => v,
        Err(_) => return CollectionOutcome::fallback(&request, ManualFallbackReason::Network),
    };
    if response.status().is_redirection() {
        return CollectionOutcome::fallback(&request, ManualFallbackReason::UnsupportedPage);
    }
    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return CollectionOutcome::fallback(&request, ManualFallbackReason::LoginRequired);
    }
    if !response.status().is_success() {
        return CollectionOutcome::fallback(&request, ManualFallbackReason::RobotsOrAccessDenied);
    }
    if response
        .content_length()
        .is_some_and(|n| n > request.max_bytes as u64)
    {
        return CollectionOutcome::fallback(&request, ManualFallbackReason::SizeLimit);
    }
    let final_url = response.url().to_string();
    let mut bytes = Vec::new();
    if response
        .take((request.max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return CollectionOutcome::fallback(&request, ManualFallbackReason::Network);
    }
    if bytes.len() > request.max_bytes {
        return CollectionOutcome::fallback(&request, ManualFallbackReason::SizeLimit);
    }
    let html = String::from_utf8_lossy(&bytes);
    let mut text = String::new();
    let mut tag = false;
    for ch in html.chars() {
        match ch {
            '<' => tag = true,
            '>' => {
                tag = false;
                text.push(' ')
            }
            _ if !tag => text.push(ch),
            _ => {}
        }
    }
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        CollectionOutcome::fallback(&request, ManualFallbackReason::EmptyContent)
    } else {
        CollectionOutcome::Collected { final_url, text }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_http_and_preserves_manual_fallback() {
        let request = UrlCollectionRequest::new("https://github.com/jobs/42").unwrap();
        assert_eq!(
            CollectionOutcome::fallback(&request, ManualFallbackReason::LoginRequired),
            CollectionOutcome::ManualInputRequired {
                original_url: "https://github.com/jobs/42".into(),
                reason: ManualFallbackReason::LoginRequired
            }
        );
    }
    #[test]
    fn rejects_embedded_credentials_and_file_urls() {
        assert_eq!(
            UrlCollectionRequest::new("https://user:pass@example.com/job"),
            Err(UrlRejection::CredentialsInUrl)
        );
        assert_eq!(
            UrlCollectionRequest::new("file:///etc/passwd"),
            Err(UrlRejection::LocalFile)
        );
        assert_eq!(
            UrlCollectionRequest::new("http://127.0.0.1/admin"),
            Err(UrlRejection::PrivateNetwork)
        );
        assert_eq!(
            UrlCollectionRequest::new("http://192.168.1.3/job"),
            Err(UrlRejection::PrivateNetwork)
        );
    }
    #[test]
    fn collection_rejects_private_target_without_network_access() {
        assert!(matches!(
            collect("http://169.254.169.254/latest/meta-data"),
            CollectionOutcome::ManualInputRequired { .. }
        ));
    }
    #[test]
    fn rejects_all_private_host_forms_and_bad_schemes() {
        for url in [
            "ftp://example.com/x",
            "not a url",
            "http://localhost/x",
            "http://x.localhost/x",
            "http://10.1.2.3/x",
            "http://172.16.1.2/x",
            "http://172.31.1.2/x",
            "http://192.168.2.3/x",
            "http://0.0.0.0/x",
            "http://[::1]/x",
        ] {
            assert!(UrlCollectionRequest::new(url).is_err(), "{url}");
        }
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.1.1",
            "0.0.0.0",
            "255.255.255.255",
            "::1",
            "::",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(private_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(!private_ip("8.8.8.8".parse().unwrap()));
        assert!(!private_ip("2001:4860:4860::8888".parse().unwrap()));
    }
    #[test]
    fn invalid_collection_returns_explicit_unsupported_fallback() {
        assert_eq!(
            collect("javascript:alert(1)"),
            CollectionOutcome::ManualInputRequired {
                original_url: "javascript:alert(1)".into(),
                reason: ManualFallbackReason::UnsupportedPage
            }
        );
    }
}
