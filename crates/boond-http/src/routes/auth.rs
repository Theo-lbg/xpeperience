use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use boond_domain::model::{LoginInput, Me, Named, PasswordChange, Preferences, SetupInput};
use boond_domain::permissions::{AreaMeta, areas_meta};
use boond_domain::refs::{RefItem, RefListMeta, lists_meta};
use boond_domain::settings::{CURRENCIES, DATE_FORMATS, LOCALES, OPTIONAL_FIELDS, OptionalField, Settings};
use boond_service::{admin, auth};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::extract::{Body, CurrentUser, clear_cookie, session_cookie, session_token};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/status", get(status))
        .route("/api/auth/setup", post(setup))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/bootstrap", get(bootstrap))
        .route("/api/me/preferences", put(preferences))
        .route("/api/me/password", put(password))
}

/// Vérifie que le serveur répond et que la base est accessible (sonde Docker).
async fn health(State(s): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    s.run(|c| {
        auth::setup_required(c)?;
        Ok(())
    })
    .await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

#[derive(Serialize)]
struct Status {
    setup_required: bool,
    setup_code_required: bool,
    authenticated: bool,
    app_name: String,
}

async fn status(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Status>> {
    let token = session_token(&headers);
    let code = s.config.setup_code.is_some();
    s.run(move |c| {
        let setup_required = auth::setup_required(c)?;
        Ok(Status {
            setup_required,
            setup_code_required: setup_required && code,
            authenticated: token.is_some_and(|t| auth::session_user(c, &t).is_ok()),
            app_name: admin::settings(c)?.app_name,
        })
    })
    .await
    .map(Json)
}

fn with_session(s: &AppState, token: &str) -> Response {
    let mut res = Json(serde_json::json!({ "ok": true })).into_response();
    res.headers_mut().insert(SET_COOKIE, session_cookie(token, s.secure()));
    res
}

fn too_many(wait: std::time::Duration) -> ApiError {
    let minutes = wait.as_secs().div_ceil(60).max(1);
    let mut e = ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "rate_limited",
        format!("Trop de tentatives. Réessayez dans {minutes} min."),
    );
    e.retry_after = Some(wait.as_secs().max(1));
    e
}

/// Comparaison en temps constant (le code d'installation est un secret).
fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    diff == 0
}

#[derive(Deserialize)]
struct SetupBody {
    #[serde(flatten)]
    input: SetupInput,
    #[serde(default)]
    setup_code: String,
}

async fn setup(
    State(s): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Body(body): Body<SetupBody>,
) -> ApiResult<Response> {
    let ip = format!("ip:{}", s.client_ip(&headers, peer));
    s.limiter.check(&ip).map_err(too_many)?;
    if let Some(expected) = &s.config.setup_code
        && s.setup_required().await?
        && !same_secret(expected.trim(), body.setup_code.trim())
    {
        s.limiter.failure(&ip);
        return Err(boond_domain::DomainError::invalid("setup_code", "Code d'installation incorrect.").into());
    }
    let input = body.input;
    let (_, token) = s.run(move |c| auth::setup(c, input)).await?;
    Ok(with_session(&s, &token))
}

async fn login(
    State(s): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Body(input): Body<LoginInput>,
) -> ApiResult<Response> {
    let ip = format!("ip:{}", s.client_ip(&headers, peer));
    let account = format!("acct:{}", input.email.trim().to_lowercase());
    s.limiter.check(&ip).map_err(too_many)?;
    s.limiter.check(&account).map_err(too_many)?;
    match s.run(move |c| auth::login(c, input)).await {
        Ok((_, token)) => {
            s.limiter.success(&account);
            Ok(with_session(&s, &token))
        }
        Err(e) => {
            // Seuls les refus d'identifiants comptent comme des échecs.
            if matches!(e.status(), StatusCode::UNPROCESSABLE_ENTITY) {
                s.limiter.failure(&ip);
                s.limiter.failure(&account);
                tracing::warn!("échec de connexion depuis {ip}");
            }
            Err(e)
        }
    }
}

async fn logout(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = session_token(&headers) {
        s.run(move |c| auth::logout(c, &token)).await?;
    }
    let mut res = Json(serde_json::json!({ "ok": true })).into_response();
    res.headers_mut().insert(SET_COOKIE, clear_cookie(s.secure()));
    Ok(res)
}

/// Tout ce dont l'interface a besoin au démarrage, en un seul appel.
#[derive(Serialize)]
struct Bootstrap {
    me: Me,
    settings: Settings,
    refs: Vec<RefItem>,
    lists: Vec<RefListMeta>,
    users: Vec<Named>,
    areas: Vec<AreaMeta>,
    optional_fields: &'static [OptionalField],
    locales: &'static [(&'static str, &'static str)],
    date_formats: &'static [&'static str],
    currencies: &'static [&'static str],
    port: u16,
    /// Adresse à saisir dans l'extension : l'adresse publique du serveur, sinon l'adresse locale.
    address: String,
    version: &'static str,
}

async fn bootstrap(State(s): State<AppState>, CurrentUser(me): CurrentUser) -> ApiResult<Json<Bootstrap>> {
    let port = s.config.port;
    let address = s.config.public_url.clone().map(|u| u.trim_end_matches('/').to_string()).unwrap_or_else(|| format!("http://127.0.0.1:{port}"));
    s.run(move |c| {
        Ok(Bootstrap {
            me: auth::me(c, &me)?,
            settings: admin::settings(c)?,
            refs: admin::refs(c)?,
            lists: lists_meta(),
            users: admin::directory(c)?,
            areas: areas_meta(),
            optional_fields: OPTIONAL_FIELDS,
            locales: LOCALES,
            date_formats: DATE_FORMATS,
            currencies: CURRENCIES,
            port,
            address,
            version: env!("CARGO_PKG_VERSION"),
        })
    })
    .await
    .map(Json)
}

async fn preferences(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Body(prefs): Body<Preferences>,
) -> ApiResult<Json<Preferences>> {
    s.run(move |c| auth::set_preferences(c, &me, prefs)).await.map(Json)
}

async fn password(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    headers: HeaderMap,
    Body(input): Body<PasswordChange>,
) -> ApiResult<Json<serde_json::Value>> {
    let token = session_token(&headers);
    s.run(move |c| auth::change_password(c, &me, input, token.as_deref())).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
