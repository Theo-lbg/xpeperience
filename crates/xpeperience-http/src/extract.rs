//! Extracteurs : utilisateur de session, utilisateur de l'extension, corps JSON.

use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue};
use xpeperience_domain::DomainError;
use xpeperience_service::AuthUser;
use serde::de::DeserializeOwned;

use crate::error::ApiError;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "xpeperience_session";
const SESSION_MAX_AGE: i64 = 365 * 24 * 3600;

pub fn session_token(headers: &HeaderMap) -> Option<String> {
    headers.get_all(COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == SESSION_COOKIE && !value.is_empty()).then(|| value.to_string())
    })
}

/// Cookie de session : inaccessible au JavaScript, jamais envoyé depuis un autre site, `Secure` en HTTPS.
pub fn session_cookie(token: &str, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!("{SESSION_COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={SESSION_MAX_AGE}{secure}"))
        .expect("cookie ASCII")
}

pub fn clear_cookie(secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0{secure}")).expect("cookie ASCII")
}

/// Utilisateur connecté à l'interface (cookie de session).
pub struct CurrentUser(pub AuthUser);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = session_token(&parts.headers).ok_or(DomainError::Unauthorized)?;
        let user = state.run(move |conn| xpeperience_service::auth::session_user(conn, &token)).await?;
        Ok(Self(user))
    }
}

/// Utilisateur de l'extension LinkedIn (jeton d'appairage `Bearer xpe_…`).
pub struct ExtensionUser(pub AuthUser);

impl FromRequestParts<AppState> for ExtensionUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .filter(|t| t.starts_with("xpe_") || t.starts_with("bxp_")) // « bxp_ » : jetons créés avant le renommage
            .map(str::to_string)
            .ok_or(DomainError::Unauthorized)?;
        let user = state.run(move |conn| xpeperience_service::auth::token_user(conn, &token)).await?;
        Ok(Self(user))
    }
}

/// Corps JSON dont les erreurs de format sont renvoyées en JSON lisible.
pub struct Body<T>(pub T);

impl<T, S> FromRequest<S> for Body<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(Self(v)),
            Err(e) => Err(ApiError::bad_request(format!("Requête invalide : {}", e.body_text()))),
        }
    }
}
