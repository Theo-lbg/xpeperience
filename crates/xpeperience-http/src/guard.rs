//! Protections appliquées à toutes les requêtes.

use axum::extract::{Request, State};
use axum::http::header::HOST;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::ApiError;
use crate::state::AppState;

/// En-tête que l'interface ajoute à chaque écriture. Une page web tierce ne peut
/// pas l'envoyer vers ce serveur sans pré-vérification CORS, que le serveur refuse.
pub const CLIENT_HEADER: &str = "x-xpeperience-client";

pub async fn protect(State(state): State<AppState>, req: Request, next: Next) -> Response {
    // Protection contre le DNS rebinding : seuls les hôtes connus sont acceptés.
    let host = req.headers().get(HOST).and_then(|h| h.to_str().ok());
    if !state.host_allowed(host) {
        return ApiError::new(StatusCode::MISDIRECTED_REQUEST, "host", "Hôte non autorisé.").into_response();
    }

    let path = req.uri().path();
    let writes = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if req.method() == Method::OPTIONS && path.starts_with("/api/") {
        // Aucune requête inter-origines n'est acceptée.
        return StatusCode::FORBIDDEN.into_response();
    }
    if writes && path.starts_with("/api/") && !path.starts_with("/api/ext/") && !req.headers().contains_key(CLIENT_HEADER) {
        return ApiError::new(StatusCode::FORBIDDEN, "csrf", "Requête refusée.").into_response();
    }

    let is_api = path.starts_with("/api/");
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert("permissions-policy", HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"));
    if state.secure() {
        h.insert("strict-transport-security", HeaderValue::from_static("max-age=31536000"));
    }
    if is_api {
        h.entry("cache-control").or_insert(HeaderValue::from_static("no-store"));
    } else {
        h.insert(
            "content-security-policy",
            HeaderValue::from_static(
                "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; \
                 script-src 'self'; connect-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'",
            ),
        );
    }
    res
}
