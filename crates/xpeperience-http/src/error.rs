use axum::Json;
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use xpeperience_domain::{DomainError, FieldError};
use serde::Serialize;

/// Erreur renvoyée à l'interface, toujours au format JSON.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
    /// Délai conseillé avant une nouvelle tentative (en secondes), pour les erreurs 429.
    pub retry_after: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<FieldError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, error: &'static str, message: impl Into<String>) -> Self {
        Self { status, body: ErrorBody { error, message: message.into(), fields: Vec::new(), details: None }, retry_after: None }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "Ressource introuvable.")
    }

    pub fn internal(detail: impl std::fmt::Display) -> Self {
        tracing::error!("erreur interne : {detail}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", "Une erreur inattendue est survenue. Réessayez.")
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        let message = e.to_string();
        match e {
            DomainError::Validation(fields) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                body: ErrorBody { error: "validation", message, fields, details: None },
                retry_after: None,
            },
            DomainError::NotFound(_) => Self::new(StatusCode::NOT_FOUND, "not_found", message),
            DomainError::Forbidden => Self::new(StatusCode::FORBIDDEN, "forbidden", message),
            DomainError::Unauthorized => Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message),
            DomainError::Conflict { details, .. } => Self {
                status: StatusCode::CONFLICT,
                body: ErrorBody { error: "conflict", message, fields: Vec::new(), details },
                retry_after: None,
            },
            DomainError::Rule(_) => Self::new(StatusCode::UNPROCESSABLE_ENTITY, "rule", message),
            DomainError::Internal(detail) => Self::internal(detail),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry = self.retry_after;
        let mut res = (self.status, Json(self.body)).into_response();
        if let Some(secs) = retry
            && let Ok(v) = HeaderValue::from_str(&secs.to_string())
        {
            res.headers_mut().insert(RETRY_AFTER, v);
        }
        res
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
