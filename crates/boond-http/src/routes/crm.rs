use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use boond_domain::Id;
use boond_domain::model::{
    Activity, ActivityFilter, ActivityInput, ArchiveInput, Company, CompanyFilter, CompanyInput, Contact, ContactFilter,
    ContactInput, Named,
};
use boond_service::{crm, projects};
use serde::Deserialize;

use crate::error::ApiResult;
use crate::extract::{Body, CurrentUser};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/companies", get(list_companies).post(create_company))
        .route("/api/companies/{id}", get(get_company).put(update_company).delete(delete_company))
        .route("/api/companies/{id}/archive", post(archive_company))
        .route("/api/companies/{id}/open-projects", get(open_projects))
        .route("/api/contacts", get(list_contacts).post(create_contact))
        .route("/api/contacts/{id}", get(get_contact).put(update_contact).delete(delete_contact))
        .route("/api/contacts/{id}/archive", post(archive_contact))
        .route("/api/activities", get(list_activities).post(create_activity))
        .route("/api/activities/{id}", put(update_activity).delete(delete_activity))
        .route("/api/activities/{id}/done", post(done_activity))
}

async fn list_companies(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(f): Query<CompanyFilter>) -> ApiResult<Json<Vec<Company>>> {
    s.run(move |c| crm::companies(c, &me, &f)).await.map(Json)
}

async fn get_company(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Company>> {
    s.run(move |c| crm::company(c, &me, id)).await.map(Json)
}

async fn create_company(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<CompanyInput>) -> ApiResult<Json<Company>> {
    s.run(move |c| crm::create_company(c, &me, i)).await.map(Json)
}

async fn update_company(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<CompanyInput>,
) -> ApiResult<Json<Company>> {
    s.run(move |c| crm::update_company(c, &me, id, i)).await.map(Json)
}

async fn archive_company(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ArchiveInput>,
) -> ApiResult<Json<Company>> {
    s.run(move |c| crm::archive_company(c, &me, id, i.archived)).await.map(Json)
}

async fn delete_company(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| crm::delete_company(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn open_projects(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Vec<Named>>> {
    s.run(move |c| projects::open_projects(c, &me, id)).await.map(Json)
}

async fn list_contacts(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(f): Query<ContactFilter>) -> ApiResult<Json<Vec<Contact>>> {
    s.run(move |c| crm::contacts(c, &me, &f)).await.map(Json)
}

async fn get_contact(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<Json<Contact>> {
    s.run(move |c| crm::contact(c, &me, id)).await.map(Json)
}

async fn create_contact(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<ContactInput>) -> ApiResult<Json<Contact>> {
    s.run(move |c| crm::create_contact(c, &me, i)).await.map(Json)
}

async fn update_contact(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ContactInput>,
) -> ApiResult<Json<Contact>> {
    s.run(move |c| crm::update_contact(c, &me, id, i)).await.map(Json)
}

async fn archive_contact(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ArchiveInput>,
) -> ApiResult<Json<Contact>> {
    s.run(move |c| crm::archive_contact(c, &me, id, i.archived)).await.map(Json)
}

async fn delete_contact(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| crm::delete_contact(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_activities(State(s): State<AppState>, CurrentUser(me): CurrentUser, Query(f): Query<ActivityFilter>) -> ApiResult<Json<Vec<Activity>>> {
    s.run(move |c| crm::activities(c, &me, &f)).await.map(Json)
}

async fn create_activity(State(s): State<AppState>, CurrentUser(me): CurrentUser, Body(i): Body<ActivityInput>) -> ApiResult<Json<Activity>> {
    s.run(move |c| crm::create_activity(c, &me, i)).await.map(Json)
}

async fn update_activity(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<ActivityInput>,
) -> ApiResult<Json<Activity>> {
    s.run(move |c| crm::update_activity(c, &me, id, i)).await.map(Json)
}

#[derive(Deserialize)]
struct Done {
    done: bool,
}

async fn done_activity(
    State(s): State<AppState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<Id>,
    Body(i): Body<Done>,
) -> ApiResult<Json<Activity>> {
    s.run(move |c| crm::set_activity_done(c, &me, id, i.done)).await.map(Json)
}

async fn delete_activity(State(s): State<AppState>, CurrentUser(me): CurrentUser, Path(id): Path<Id>) -> ApiResult<StatusCode> {
    s.run(move |c| crm::delete_activity(c, &me, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
