use crate::application::ports::ApplicationError;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("storage operation failed")]
    Io(#[from] std::io::Error),
    #[error("database is already in use")]
    DatabaseLocked,
    #[error("database integrity check failed: {0}")]
    Integrity(String),
    #[error("unsupported database schema; missing tables: {0:?}")]
    IncompatibleSchema(Vec<String>),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("internal error")]
    Internal,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Validation,
    NotFound,
    Conflict,
    Cancelled,
    Unavailable,
    Internal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Envelope<T: Serialize> {
    Ok { success: bool, data: T },
    Error { success: bool, error: ErrorBody },
}
impl<T: Serialize> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self::Ok {
            success: true,
            data,
        }
    }
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Error {
            success: false,
            error: ErrorBody {
                code,
                message: message.into(),
                details: None,
            },
        }
    }
    pub fn unsupported(command: &str) -> Self {
        Self::error(
            ErrorCode::Unavailable,
            format!("unsupported command: {command}"),
        )
    }
}
impl From<ApplicationError> for AppError {
    fn from(v: ApplicationError) -> Self {
        match v {
            ApplicationError::Validation(x) => Self::Validation(x),
            ApplicationError::NotFound(x) => Self::NotFound(x),
            ApplicationError::Conflict(x) => Self::Conflict(x),
            ApplicationError::Cancelled => Self::Cancelled,
            ApplicationError::Unavailable(x) => Self::Unavailable(x),
            ApplicationError::Internal => Self::Internal,
        }
    }
}
impl<T: Serialize> From<Result<T, AppError>> for Envelope<T> {
    fn from(v: Result<T, AppError>) -> Self {
        match v {
            Ok(x) => Self::ok(x),
            Err(e) => {
                let c = match &e {
                    AppError::Validation(_) => ErrorCode::Validation,
                    AppError::NotFound(_) => ErrorCode::NotFound,
                    AppError::Conflict(_) => ErrorCode::Conflict,
                    AppError::Cancelled => ErrorCode::Cancelled,
                    AppError::Unavailable(_) | AppError::DatabaseLocked => ErrorCode::Unavailable,
                    _ => ErrorCode::Internal,
                };
                Self::error(c, e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn code(error: AppError) -> ErrorCode {
        let value = serde_json::to_value(Envelope::<()>::from(Err(error))).unwrap();
        serde_json::from_value(value["error"]["code"].clone()).unwrap()
    }
    #[test]
    fn maps_every_public_error_family() {
        assert_eq!(
            code(AppError::Validation("x".into())),
            ErrorCode::Validation
        );
        assert_eq!(code(AppError::NotFound("x".into())), ErrorCode::NotFound);
        assert_eq!(code(AppError::Conflict("x".into())), ErrorCode::Conflict);
        assert_eq!(code(AppError::Cancelled), ErrorCode::Cancelled);
        assert_eq!(
            code(AppError::Unavailable("x".into())),
            ErrorCode::Unavailable
        );
        assert_eq!(code(AppError::DatabaseLocked), ErrorCode::Unavailable);
        assert_eq!(code(AppError::Integrity("x".into())), ErrorCode::Internal);
        assert_eq!(
            code(AppError::IncompatibleSchema(vec!["x".into()])),
            ErrorCode::Internal
        );
        assert_eq!(
            code(AppError::Io(std::io::Error::other("x"))),
            ErrorCode::Internal
        );
        assert_eq!(
            code(AppError::Database(rusqlite::Error::InvalidQuery)),
            ErrorCode::Internal
        );
        assert_eq!(code(AppError::Internal), ErrorCode::Internal);
    }
    #[test]
    fn maps_application_errors_and_helpers() {
        let values = [
            ApplicationError::Validation("v".into()),
            ApplicationError::NotFound("n".into()),
            ApplicationError::Conflict("c".into()),
            ApplicationError::Cancelled,
            ApplicationError::Unavailable("u".into()),
            ApplicationError::Internal,
        ];
        let codes = values
            .into_iter()
            .map(|e| code(e.into()))
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            vec![
                ErrorCode::Validation,
                ErrorCode::NotFound,
                ErrorCode::Conflict,
                ErrorCode::Cancelled,
                ErrorCode::Unavailable,
                ErrorCode::Internal
            ]
        );
        let ok = serde_json::to_value(Envelope::ok(7)).unwrap();
        assert_eq!(ok["data"], 7);
        let unsupported = serde_json::to_value(Envelope::<()>::unsupported("x")).unwrap();
        assert_eq!(unsupported["error"]["code"], "UNAVAILABLE");
    }
}
