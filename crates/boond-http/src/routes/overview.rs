use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use boond_domain::model::{Dashboard, EntityKind, HistoryEntry, SearchHit};
use boond_domain::Id;
use boond_service::overview;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::extract::CurrentUser;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/dashboard", get(dashboard))
        .route("/api/search", get(search))
        .route("/api/history/{entity}/{id}", get(history))
}

async fn dashboard(State(s): State<AppState>, CurrentUser(me): CurrentUser) -> ApiResult<Json<Dashboard>> {
    s.run(move |c| overview::dashboard(c, &me)).await.map(Json)
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(q): Query<SearchQuery>) -> ApiResult<Json<Vec<SearchHit>>> {
    s.run(move |c| overview::search(c, &me, &q.q)).await.map(Json)
}

async fn history(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path((entity, id)): Path<(String, Id)>,
) -> ApiResult<Json<Vec<HistoryEntry>>> {
    let entity = EntityKind::from_code(&entity).ok_or_else(ApiError::not_found)?;
    s.run(move |c| overview::history(c, &me, entity, id)).await.map(Json)
}
