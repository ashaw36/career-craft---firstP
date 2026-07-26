//! OS credential storage. There is deliberately no plaintext fallback.
use crate::application::ports::{ApplicationError, SecretStore};
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsCredentialStore;
impl WindowsCredentialStore {
    const SERVICE: &'static str = "com.careercraft.desktop";
    fn validate(provider: &str) -> Result<(), ApplicationError> {
        if provider.trim().is_empty() {
            Err(ApplicationError::Validation("provider is required".into()))
        } else {
            Ok(())
        }
    }
}
impl SecretStore for WindowsCredentialStore {
    fn put(&self, p: &str, s: &str) -> Result<(), ApplicationError> {
        Self::validate(p)?;
        if s.is_empty() {
            return Err(ApplicationError::Validation("secret is required".into()));
        }
        put(p, s)
    }
    fn get(&self, p: &str) -> Result<String, ApplicationError> {
        Self::validate(p)?;
        get(p)
    }
    fn exists(&self, p: &str) -> Result<bool, ApplicationError> {
        Self::validate(p)?;
        exists(p)
    }
    fn delete(&self, p: &str) -> Result<(), ApplicationError> {
        Self::validate(p)?;
        delete(p)
    }
}
#[cfg(windows)]
fn entry(p: &str) -> Result<keyring::Entry, ApplicationError> {
    keyring::Entry::new(WindowsCredentialStore::SERVICE, p)
        .map_err(|_| ApplicationError::Unavailable("Windows Credential Manager unavailable".into()))
}
#[cfg(windows)]
fn put(p: &str, s: &str) -> Result<(), ApplicationError> {
    entry(p)?
        .set_password(s)
        .map_err(|_| ApplicationError::Unavailable("failed to store credential".into()))
}
#[cfg(windows)]
fn get(p: &str) -> Result<String, ApplicationError> {
    entry(p)?.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => ApplicationError::NotFound("credential".into()),
        _ => ApplicationError::Unavailable("failed to read credential".into()),
    })
}
#[cfg(windows)]
fn exists(p: &str) -> Result<bool, ApplicationError> {
    match entry(p)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(_) => Err(ApplicationError::Unavailable(
            "failed to query credential".into(),
        )),
    }
}
#[cfg(windows)]
fn delete(p: &str) -> Result<(), ApplicationError> {
    match entry(p)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(ApplicationError::Unavailable(
            "failed to delete credential".into(),
        )),
    }
}
#[cfg(not(windows))]
fn put(_: &str, _: &str) -> Result<(), ApplicationError> {
    Err(ApplicationError::Unavailable(
        "Windows Credential Manager is required".into(),
    ))
}
#[cfg(not(windows))]
fn get(_: &str) -> Result<String, ApplicationError> {
    Err(ApplicationError::Unavailable(
        "Windows Credential Manager is required".into(),
    ))
}
#[cfg(not(windows))]
fn exists(_: &str) -> Result<bool, ApplicationError> {
    Err(ApplicationError::Unavailable(
        "Windows Credential Manager is required".into(),
    ))
}
#[cfg(not(windows))]
fn delete(_: &str) -> Result<(), ApplicationError> {
    Err(ApplicationError::Unavailable(
        "Windows Credential Manager is required".into(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_blank_provider() {
        assert!(matches!(
            WindowsCredentialStore.exists(" "),
            Err(ApplicationError::Validation(_))
        ));
    }
}
