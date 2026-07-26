//! Signed updater policy/state machine. Transport/install adapters remain Tauri-owned.
use serde::Deserialize;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available,
    Downloading,
    Verifying,
    Staged,
    Applying,
    Succeeded,
    Failed(String),
    RolledBack,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignedUpdateManifest {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub signature: String,
}
pub trait SignatureVerifier {
    fn verify(&self, manifest: &SignedUpdateManifest) -> bool;
}
pub struct UpdateMachine {
    state: UpdateState,
    previous_version: Option<String>,
}
impl Default for UpdateMachine {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            previous_version: None,
        }
    }
}
impl UpdateMachine {
    pub fn trusted_transport_staged(&mut self) -> Result<(), &'static str> {
        if self.state != UpdateState::Downloading {
            return Err("invalid update transition");
        }
        self.state = UpdateState::Verifying;
        self.state = UpdateState::Staged;
        Ok(())
    }
}
impl UpdateMachine {
    pub fn state(&self) -> &UpdateState {
        &self.state
    }
    pub fn begin_check(&mut self) -> Result<(), &'static str> {
        if self.state != UpdateState::Idle {
            return Err("update already active");
        }
        self.state = UpdateState::Checking;
        Ok(())
    }
    pub fn available(&mut self) -> Result<(), &'static str> {
        if self.state != UpdateState::Checking {
            return Err("invalid update transition");
        }
        self.state = UpdateState::Available;
        Ok(())
    }
    pub fn begin_download(&mut self) -> Result<(), &'static str> {
        if self.state != UpdateState::Available {
            return Err("invalid update transition");
        }
        self.state = UpdateState::Downloading;
        Ok(())
    }
    pub fn verify_and_stage(
        &mut self,
        manifest: &SignedUpdateManifest,
        verifier: &dyn SignatureVerifier,
    ) -> Result<(), &'static str> {
        if self.state != UpdateState::Downloading {
            return Err("invalid update transition");
        }
        self.state = UpdateState::Verifying;
        let https = reqwest::Url::parse(&manifest.url)
            .ok()
            .is_some_and(|u| u.scheme() == "https" && u.host_str().is_some());
        let digest =
            manifest.sha256.len() == 64 && manifest.sha256.bytes().all(|b| b.is_ascii_hexdigit());
        if manifest.version.trim().is_empty()
            || manifest.signature.trim().is_empty()
            || !https
            || !digest
            || !verifier.verify(manifest)
        {
            self.state =
                UpdateState::Failed("manifest signature or metadata verification failed".into());
            return Err("update verification failed");
        }
        self.state = UpdateState::Staged;
        Ok(())
    }
    pub fn begin_apply(&mut self, current_version: &str) -> Result<(), &'static str> {
        if self.state != UpdateState::Staged {
            return Err("invalid update transition");
        }
        self.previous_version = Some(current_version.into());
        self.state = UpdateState::Applying;
        Ok(())
    }
    pub fn applied(&mut self) -> Result<(), &'static str> {
        if self.state != UpdateState::Applying {
            return Err("invalid update transition");
        }
        self.state = UpdateState::Succeeded;
        Ok(())
    }
    pub fn apply_failed_and_rollback(&mut self) -> Result<&str, &'static str> {
        if self.state != UpdateState::Applying {
            return Err("invalid update transition");
        }
        let previous = self
            .previous_version
            .as_deref()
            .ok_or("rollback version missing")?;
        self.state = UpdateState::RolledBack;
        Ok(previous)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct V(bool);
    impl SignatureVerifier for V {
        fn verify(&self, _: &SignedUpdateManifest) -> bool {
            self.0
        }
    }
    fn manifest() -> SignedUpdateManifest {
        SignedUpdateManifest {
            version: "1.2.0".into(),
            url: "https://updates.example/app".into(),
            sha256: "a".repeat(64),
            signature: "minisign-signature".into(),
        }
    }
    #[test]
    fn rejects_unsigned_tampered_or_insecure_manifest() {
        for mut value in [manifest(), manifest(), manifest()] {
            let mut machine = UpdateMachine::default();
            machine.begin_check().unwrap();
            machine.available().unwrap();
            machine.begin_download().unwrap();
            value.url = "http://updates.example/app".into();
            assert!(machine.verify_and_stage(&value, &V(true)).is_err());
            assert!(matches!(machine.state(), UpdateState::Failed(_)))
        }
        let mut machine = UpdateMachine::default();
        machine.begin_check().unwrap();
        machine.available().unwrap();
        machine.begin_download().unwrap();
        assert!(machine.verify_and_stage(&manifest(), &V(false)).is_err())
    }
    #[test]
    fn successful_apply_and_failed_apply_rollback_are_explicit() {
        let mut success = UpdateMachine::default();
        success.begin_check().unwrap();
        success.available().unwrap();
        success.begin_download().unwrap();
        success.verify_and_stage(&manifest(), &V(true)).unwrap();
        success.begin_apply("1.1.0").unwrap();
        success.applied().unwrap();
        assert_eq!(success.state(), &UpdateState::Succeeded);
        let mut failed = UpdateMachine::default();
        failed.begin_check().unwrap();
        failed.available().unwrap();
        failed.begin_download().unwrap();
        failed.verify_and_stage(&manifest(), &V(true)).unwrap();
        failed.begin_apply("1.1.0").unwrap();
        assert_eq!(failed.apply_failed_and_rollback().unwrap(), "1.1.0");
        assert_eq!(failed.state(), &UpdateState::RolledBack)
    }
}
