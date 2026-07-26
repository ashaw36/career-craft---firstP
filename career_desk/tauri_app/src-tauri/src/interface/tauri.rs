use crate::error::Envelope;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub healthy: bool,
    pub service: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn health() -> Envelope<HealthResponse> {
    Envelope::ok(HealthResponse {
        healthy: true,
        service: "careercraft-core",
    })
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn version() -> Envelope<VersionResponse> {
    Envelope::ok(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn health_uses_success_envelope() {
        let json = serde_json::to_value(health()).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["healthy"], true);
    }
    #[test]
    fn version_uses_package_version() {
        let json = serde_json::to_value(version()).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"));
    }
}
