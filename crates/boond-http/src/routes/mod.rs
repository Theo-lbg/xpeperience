//! Routes de l'API. Chaque gestionnaire délègue à `boond-service`, qui vérifie les droits.

mod admin;
mod auth;
mod crm;
mod ext;
mod needs;
mod overview;
mod projects;
mod resources;

use axum::extract::DefaultBodyLimit;
use axum::{Router, middleware};

use crate::state::AppState;
use crate::{assets, guard};

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(admin::routes())
        .merge(crm::routes())
        .merge(needs::routes())
        .merge(resources::routes())
        .merge(projects::routes())
        .merge(overview::routes())
        .merge(ext::routes())
        .fallback(assets::spa)
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), guard::protect))
        .with_state(state)
}
