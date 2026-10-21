use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use xpeperience_domain::model::{
    ArchiveInput, HrContract, HrContractInput, Resource, ResourceFile, ResourceFilter, ResourceInput,
};
use xpeperience_domain::{DomainError, Id};
use xpeperience_service::resources;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::extract::{Body, CurrentUser};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/resources", get(list_resources).post(create_resource))
        .route("/api/resources/{id}", get(get_resource).put(update_resource).delete(delete_resource))
        .route("/api/resources/{id}/archive", post(archive_resource))
        .route("/api/resources/{id}/convert", post(convert_resource))
        .route("/api/resources/{id}/contracts", get(list_contracts).post(create_contract))
        .route("/api/contracts/{id}", put(update_contract).delete(delete_contract))
        .route("/api/resources/{id}/files", get(list_files).post(upload_file))
        .route("/api/files/{id}", get(download_file).delete(delete_file))
}

async fn list_resources(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(f): Query<ResourceFilter>) -> ApiResult<Json<Vec<Resource>>> {
    s.run(move |c| resources::resources(c, &me, &f)).await.map(Json)
}

async fn get_resource(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Resource>> {
    s.run(move |c| resources::resource(c, &me, id)).await.map(Json)
}

async fn create_resource(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<ResourceInput>) -> ApiResult<Json<Resource>> {
    s.run(move |c| resources::create_resource(c, &me, i)).await.map(Json)
}

async fn update_resource(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ResourceInput>,
) -> ApiResult<Json<Resource>> {
    s.run(move |c| resources::update_resource(c, &me, id, i)).await.map(Json)
}

async fn archive_resource(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ArchiveInput>,
) -> ApiResult<Json<Resource>> {
    s.run(move |c| resources::archive_resource(c, &me, id, i.archived)).await.map(Json)
}

async fn convert_resource(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Resource>> {
    s.run(move |c| resources::convert_to_consultant(c, &me, id)).await.map(Json)
}

async fn delete_resource(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    let files = s.run(move |c| resources::delete_resource(c, &me, id)).await?;
    for f in files {
        let _ = tokio::fs::remove_file(s.documents_dir().join(&f.stored_name)).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_contracts(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Vec<HrContract>>> {
    s.run(move |c| resources::contracts(c, &me, id)).await.map(Json)
}

async fn create_contract(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<HrContractInput>,
) -> ApiResult<Json<HrContract>> {
    s.run(move |c| resources::create_contract(c, &me, id, i)).await.map(Json)
}

async fn update_contract(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<HrContractInput>,
) -> ApiResult<Json<HrContract>> {
    s.run(move |c| resources::update_contract(c, &me, id, i)).await.map(Json)
}

async fn delete_contract(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| resources::delete_contract(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ───────────────────────────── Documents ─────────────────────────────

async fn list_files(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Vec<ResourceFile>>> {
    s.run(move |c| resources::files(c, &me, id)).await.map(Json)
}

#[derive(Deserialize)]
struct UploadQuery {
    kind: String,
}

/// Envoi brut : le corps est le fichier, son nom est dans l'en-tête `X-Filename` (encodé URL).
async fn upload_file(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(resource_id): Path<Id>,
    Query(q): Query<UploadQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<ResourceFile>> {
    let mime = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    let raw_name = headers.get("x-filename").and_then(|v| v.to_str().ok()).unwrap_or("document");
    let filename = resources::sanitize_filename(&urlencoding::decode(raw_name).map(|n| n.into_owned()).unwrap_or_default());
    let size = body.len();

    let (check_me, kind, check_mime) = (me.clone(), q.kind.clone(), mime.clone());
    s.run(move |c| resources::check_upload(c, &check_me, resource_id, &kind, &check_mime, size)).await?;

    let stored = uuid::Uuid::new_v4().to_string();
    let path = s.documents_dir().join(&stored);
    tokio::fs::write(&path, &body).await.map_err(ApiError::internal)?;
    let saved = s.run(move |c| resources::add_file(c, &me, resource_id, &q.kind, &filename, &stored, &mime, size)).await;
    if saved.is_err() {
        let _ = tokio::fs::remove_file(&path).await;
    }
    saved.map(Json)
}

#[derive(Deserialize)]
struct DownloadQuery {
    inline: Option<bool>,
}

async fn download_file(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Query(q): Query<DownloadQuery>,
) -> ApiResult<Response> {
    let f = s.run(move |c| resources::file(c, &me, id)).await?;
    let bytes = tokio::fs::read(s.documents_dir().join(&f.stored_name))
        .await
        .map_err(|_| ApiError::from(DomainError::NotFound("Fichier")))?;
    let mut res = (StatusCode::OK, bytes).into_response();
    let headers = res.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str(&f.mime).unwrap_or(HeaderValue::from_static("application/octet-stream")));
    let disposition = if q.inline.unwrap_or(false) { "inline" } else { "attachment" };
    let ascii: String = f.filename.chars().map(|c| if c.is_ascii_alphanumeric() || ".-_ ".contains(c) { c } else { '_' }).collect();
    let value = format!("{disposition}; filename=\"{ascii}\"; filename*=UTF-8''{}", urlencoding::encode(&f.filename));
    if let Ok(v) = HeaderValue::from_str(&value) {
        headers.insert(CONTENT_DISPOSITION, v);
    }
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'none'; sandbox"));
    Ok(res)
}

async fn delete_file(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    let f = s.run(move |c| resources::remove_file(c, &me, id)).await?;
    let _ = tokio::fs::remove_file(s.documents_dir().join(&f.stored_name)).await;
    Ok(StatusCode::NO_CONTENT)
}
