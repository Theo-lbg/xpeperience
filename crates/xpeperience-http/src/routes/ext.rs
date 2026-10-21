//! API de l'extension LinkedIn, authentifiée par jeton d'appairage.

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use xpeperience_domain::model::Named;
use xpeperience_service::linkedin::{self, ImportRequest, Imported, Lookup, Profile, Status};
use serde::{Deserialize, Serialize};

use crate::error::ApiResult;
use crate::extract::{Body, ExtensionUser};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ext/status", get(status))
        .route("/api/ext/lookup", post(lookup))
        .route("/api/ext/companies", get(companies))
        .route("/api/ext/import", post(import))
}

async fn status(State(s): State<AppState>, ExtensionUser(me): ExtensionUser) -> ApiResult<Json<Status>> {
    s.run(move |c| linkedin::status(c, &me)).await.map(Json)
}

async fn lookup(State(s): State<AppState>, ExtensionUser(me): ExtensionUser, Body(p): Body<Profile>) -> ApiResult<Json<Lookup>> {
    s.run(move |c| linkedin::lookup(c, &me, &p)).await.map(Json)
}

#[derive(Deserialize)]
struct CompanyQuery {
    q: String,
}

async fn companies(State(s): State<AppState>, ExtensionUser(me): ExtensionUser, Query(q): Query<CompanyQuery>) -> ApiResult<Json<Vec<Named>>> {
    s.run(move |c| linkedin::companies(c, &me, &q.q)).await.map(Json)
}

#[derive(Serialize)]
struct ImportResponse {
    #[serde(flatten)]
    imported: Imported,
    /// Lien direct vers la fiche dans l'application.
    url: String,
}

async fn import(State(s): State<AppState>, ExtensionUser(me): ExtensionUser, Body(req): Body<ImportRequest>) -> ApiResult<Json<ImportResponse>> {
    let port = s.config.port;
    let imported = s.run(move |c| linkedin::import(c, &me, req)).await?;
    let url = format!("http://127.0.0.1:{port}/crm/contacts/{}", imported.contact.id);
    Ok(Json(ImportResponse { imported, url }))
}
