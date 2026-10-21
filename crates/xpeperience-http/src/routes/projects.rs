use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use xpeperience_domain::Id;
use xpeperience_domain::model::{
    ArchiveInput, Delivery, DeliveryFilter, DeliveryInput, ExtensionInput, Project, ProjectFilter, ProjectInput,
};
use xpeperience_service::projects;

use crate::error::ApiResult;
use crate::extract::{Body, CurrentUser};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}", get(get_project).put(update_project).delete(delete_project))
        .route("/api/projects/{id}/archive", post(archive_project))
        .route("/api/projects/{id}/deliveries", post(create_delivery))
        .route("/api/deliveries", get(list_deliveries))
        .route("/api/deliveries/{id}", put(update_delivery).delete(delete_delivery))
        .route("/api/deliveries/{id}/extensions", post(create_extension))
        .route("/api/extensions/{id}", put(update_extension).delete(delete_extension))
}

async fn list_projects(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(f): Query<ProjectFilter>) -> ApiResult<Json<Vec<Project>>> {
    s.run(move |c| projects::projects(c, &me, &f)).await.map(Json)
}

async fn get_project(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Project>> {
    s.run(move |c| projects::project(c, &me, id)).await.map(Json)
}

async fn create_project(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<ProjectInput>) -> ApiResult<Json<Project>> {
    s.run(move |c| projects::create_project(c, &me, i)).await.map(Json)
}

async fn update_project(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ProjectInput>,
) -> ApiResult<Json<Project>> {
    s.run(move |c| projects::update_project(c, &me, id, i)).await.map(Json)
}

async fn archive_project(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ArchiveInput>,
) -> ApiResult<Json<Project>> {
    s.run(move |c| projects::archive_project(c, &me, id, i.archived)).await.map(Json)
}

async fn delete_project(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| projects::delete_project(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_deliveries(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Query(f): Query<DeliveryFilter>,
) -> ApiResult<Json<Vec<Delivery>>> {
    s.run(move |c| projects::deliveries(c, &me, &f)).await.map(Json)
}

async fn create_delivery(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<DeliveryInput>,
) -> ApiResult<Json<Delivery>> {
    s.run(move |c| projects::create_delivery(c, &me, id, i)).await.map(Json)
}

async fn update_delivery(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<DeliveryInput>,
) -> ApiResult<Json<Delivery>> {
    s.run(move |c| projects::update_delivery(c, &me, id, i)).await.map(Json)
}

async fn delete_delivery(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| projects::delete_delivery(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_extension(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ExtensionInput>,
) -> ApiResult<Json<Delivery>> {
    s.run(move |c| projects::create_extension(c, &me, id, i)).await.map(Json)
}

async fn update_extension(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ExtensionInput>,
) -> ApiResult<Json<Delivery>> {
    s.run(move |c| projects::update_extension(c, &me, id, i)).await.map(Json)
}

async fn delete_extension(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Delivery>> {
    s.run(move |c| projects::delete_extension(c, &me, id)).await.map(Json)
}
