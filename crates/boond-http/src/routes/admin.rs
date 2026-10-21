use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use boond_domain::model::{ApiToken, CreatedToken, Role, RoleInput, User, UserInput};
use boond_domain::permissions::{Action, Area};
use boond_domain::refs::{RefItem, RefItemInput, RefList};
use boond_domain::settings::Settings;
use boond_domain::{DomainError, Id};
use boond_service::admin;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::extract::{Body, CurrentUser};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/settings/logo", put(put_logo).delete(delete_logo))
        .route("/api/branding/logo", get(get_logo))
        .route("/api/refs", get(list_refs))
        .route("/api/refs/{list}", post(create_ref))
        .route("/api/refs/{list}/order", post(reorder_refs))
        .route("/api/refs/item/{id}", put(update_ref).delete(delete_ref))
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/{id}", put(update_user))
        .route("/api/users/{id}/password", post(reset_password))
        .route("/api/roles", get(list_roles).post(create_role))
        .route("/api/roles/{id}", put(update_role).delete(delete_role))
        .route("/api/tokens", get(list_tokens).post(create_token))
        .route("/api/tokens/{id}", delete(revoke_token))
}

async fn get_settings(State(s): State<AppState>, CurrentUser(_): CurrentUser) -> ApiResult<Json<Settings>> {
    s.run(|c| admin::settings(c)).await.map(Json)
}

async fn put_settings(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(input): Body<Settings>) -> ApiResult<Json<Settings>> {
    s.run(move |c| admin::update_settings(c, &me, input)).await.map(Json)
}

// ───────────────────────────── Logo ─────────────────────────────

const LOGO_TYPES: &[(&str, &str)] = &[("image/png", "png"), ("image/jpeg", "jpg"), ("image/webp", "webp")];
const LOGO_MAX: usize = 2 * 1024 * 1024;

/// Logo personnalisé déposé dans les réglages ; 404 si l'interface doit utiliser le logo fourni.
async fn get_logo(State(s): State<AppState>) -> Response {
    let dir = s.branding_dir();
    for (mime, ext) in LOGO_TYPES {
        if let Ok(bytes) = tokio::fs::read(dir.join(format!("logo.{ext}"))).await {
            let mut res = (StatusCode::OK, bytes).into_response();
            res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(mime));
            res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
            return res;
        }
    }
    ApiError::not_found().into_response()
}

async fn put_logo(State(s): State<AppState>, CurrentUser(me): CurrentUser, headers: HeaderMap, body: Bytes) -> ApiResult<Json<serde_json::Value>> {
    me.require(Area::Settings, Action::Update)?;
    let mime = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or_default();
    let Some((_, ext)) = LOGO_TYPES.iter().find(|(m, _)| *m == mime) else {
        return Err(DomainError::invalid("logo", "Format accepté : PNG, JPEG ou WebP.").into());
    };
    if body.is_empty() || body.len() > LOGO_MAX {
        return Err(DomainError::invalid("logo", "Le logo doit faire moins de 2 Mo.").into());
    }
    let dir = s.branding_dir();
    for (_, other) in LOGO_TYPES {
        let _ = tokio::fs::remove_file(dir.join(format!("logo.{other}"))).await;
    }
    tokio::fs::write(dir.join(format!("logo.{ext}")), &body).await.map_err(ApiError::internal)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn delete_logo(State(s): State<AppState>, CurrentUser(me): CurrentUser) -> ApiResult<Json<serde_json::Value>> {
    me.require(Area::Settings, Action::Update)?;
    for (_, ext) in LOGO_TYPES {
        let _ = tokio::fs::remove_file(s.branding_dir().join(format!("logo.{ext}"))).await;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ─────────────────────── Listes paramétrables ───────────────────────

fn list_from(code: &str) -> ApiResult<RefList> {
    RefList::from_code(code).ok_or_else(ApiError::not_found)
}

async fn list_refs(State(s): State<AppState>, CurrentUser(_): CurrentUser) -> ApiResult<Json<Vec<RefItem>>> {
    s.run(|c| admin::refs(c)).await.map(Json)
}

async fn create_ref(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(list): Path<String>,
    Body(input): Body<RefItemInput>,
) -> ApiResult<Json<RefItem>> {
    let list = list_from(&list)?;
    s.run(move |c| admin::create_ref(c, &me, list, input)).await.map(Json)
}

#[derive(Deserialize)]
struct Order {
    ids: Vec<Id>,
}

async fn reorder_refs(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(list): Path<String>,
    Body(order): Body<Order>,
) -> ApiResult<Json<Vec<RefItem>>> {
    let list = list_from(&list)?;
    s.run(move |c| admin::reorder_refs(c, &me, list, order.ids)).await.map(Json)
}

async fn update_ref(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(input): Body<RefItemInput>,
) -> ApiResult<Json<RefItem>> {
    s.run(move |c| admin::update_ref(c, &me, id, input)).await.map(Json)
}

async fn delete_ref(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| admin::delete_ref(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────── Utilisateurs et rôles ───────────────────────

async fn list_users(State(s): State<AppState>, CurrentUser(me): CurrentUser) -> ApiResult<Json<Vec<User>>> {
    s.run(move |c| admin::users(c, &me)).await.map(Json)
}

async fn create_user(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(input): Body<UserInput>) -> ApiResult<Json<User>> {
    s.run(move |c| admin::create_user(c, &me, input)).await.map(Json)
}

async fn update_user(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(input): Body<UserInput>,
) -> ApiResult<Json<User>> {
    s.run(move |c| admin::update_user(c, &me, id, input)).await.map(Json)
}

#[derive(Deserialize)]
struct NewPassword {
    password: String,
}

async fn reset_password(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(input): Body<NewPassword>,
) -> ApiResult<StatusCode> {
    s.run(move |c| admin::reset_password(c, &me, id, &input.password)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_roles(State(s): State<AppState>, CurrentUser(me): CurrentUser) -> ApiResult<Json<Vec<Role>>> {
    s.run(move |c| admin::roles(c, &me)).await.map(Json)
}

async fn create_role(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(input): Body<RoleInput>) -> ApiResult<Json<Role>> {
    s.run(move |c| admin::create_role(c, &me, input)).await.map(Json)
}

async fn update_role(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(input): Body<RoleInput>,
) -> ApiResult<Json<Role>> {
    s.run(move |c| admin::update_role(c, &me, id, input)).await.map(Json)
}

async fn delete_role(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| admin::delete_role(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────── Jetons d'appairage ───────────────────────

async fn list_tokens(State(s): State<AppState>, CurrentUser(me): CurrentUser) -> ApiResult<Json<Vec<ApiToken>>> {
    s.run(move |c| admin::tokens(c, &me)).await.map(Json)
}

#[derive(Deserialize)]
struct TokenName {
    name: Option<String>,
}

async fn create_token(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(input): Body<TokenName>) -> ApiResult<Json<CreatedToken>> {
    s.run(move |c| admin::create_token(c, &me, input.name)).await.map(Json)
}

async fn revoke_token(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| admin::revoke_token(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
