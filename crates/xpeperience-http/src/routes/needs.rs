use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use xpeperience_domain::Id;
use xpeperience_domain::model::{
    ArchiveInput, ConvertInput, Need, NeedDraft, NeedDraftInput, NeedFilter, NeedInput, Positioning, PositioningFilter, PositioningInput,
    PositioningSaved, Project, StateChange, Suggestion,
};
use xpeperience_service::{needs, positionings};

use crate::error::ApiResult;
use crate::extract::{Body, CurrentUser};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/needs", get(list_needs).post(create_need))
        .route("/api/needs/draft", post(draft_need))
        .route("/api/needs/{id}", get(get_need).put(update_need).delete(delete_need))
        .route("/api/needs/{id}/archive", post(archive_need))
        .route("/api/needs/{id}/state", put(set_state))
        .route("/api/needs/{id}/convert", post(convert_need))
        .route("/api/needs/{id}/suggestions", get(suggestions))
        .route("/api/positionings", get(list_positionings).post(create_positioning))
        .route("/api/positionings/{id}", get(get_positioning).put(update_positioning).delete(delete_positioning))
        .route("/api/positionings/{id}/convert", post(convert_positioning))
}

async fn list_needs(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(f): Query<NeedFilter>) -> ApiResult<Json<Vec<Need>>> {
    s.run(move |c| needs::needs(c, &me, &f)).await.map(Json)
}

async fn draft_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<NeedDraftInput>) -> ApiResult<Json<NeedDraft>> {
    s.run(move |c| needs::draft_from_text(c, &me, &i.text)).await.map(Json)
}

async fn get_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Need>> {
    s.run(move |c| needs::need(c, &me, id)).await.map(Json)
}

async fn create_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<NeedInput>) -> ApiResult<Json<Need>> {
    s.run(move |c| needs::create_need(c, &me, i)).await.map(Json)
}

async fn update_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>, Body(i): Body<NeedInput>) -> ApiResult<Json<Need>> {
    s.run(move |c| needs::update_need(c, &me, id, i)).await.map(Json)
}

async fn set_state(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>, Body(i): Body<StateChange>) -> ApiResult<Json<Need>> {
    s.run(move |c| needs::set_state(c, &me, id, i.state_id)).await.map(Json)
}

async fn archive_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>, Body(i): Body<ArchiveInput>) -> ApiResult<Json<Need>> {
    s.run(move |c| needs::archive_need(c, &me, id, i.archived)).await.map(Json)
}

async fn delete_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| needs::delete_need(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn convert_need(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Project>> {
    s.run(move |c| needs::convert_need(c, &me, id)).await.map(Json)
}

async fn suggestions(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Vec<Suggestion>>> {
    s.run(move |c| positionings::suggestions(c, &me, id)).await.map(Json)
}

async fn list_positionings(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(f): Query<PositioningFilter>,
) -> ApiResult<Json<Vec<Positioning>>> {
    s.run(move |c| positionings::positionings(c, &me, &f)).await.map(Json)
}

async fn get_positioning(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Positioning>> {
    s.run(move |c| positionings::positioning(c, &me, id)).await.map(Json)
}

async fn create_positioning(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Body(i): Body<PositioningInput>,
) -> ApiResult<Json<PositioningSaved>> {
    s.run(move |c| positionings::create_positioning(c, &me, i)).await.map(Json)
}

async fn update_positioning(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<PositioningInput>,
) -> ApiResult<Json<PositioningSaved>> {
    s.run(move |c| positionings::update_positioning(c, &me, id, i)).await.map(Json)
}

async fn delete_positioning(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| positionings::delete_positioning(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn convert_positioning(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ConvertInput>,
) -> ApiResult<Json<Project>> {
    s.run(move |c| positionings::convert_positioning(c, &me, id, i)).await.map(Json)
}
