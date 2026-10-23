use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    status: u16,
    error: &'a str,
    message: &'a str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, label) = match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "Bad Request"),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "Not Found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "Conflict"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
        };
        let message = self.to_string();
        if status.is_server_error() {
            tracing::error!(%message, "request failed");
        }
        (
            status,
            Json(ErrorBody {
                status: status.as_u16(),
                error: label,
                message: &message,
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
pub fn bad(message: impl Into<String>) -> ApiError {
    ApiError::BadRequest(message.into())
}
pub fn require(condition: bool, message: impl Into<String>) -> ApiResult<()> {
    if condition {
        Ok(())
    } else {
        Err(bad(message))
    }
}
