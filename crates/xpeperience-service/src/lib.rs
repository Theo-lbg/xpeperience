//! Cas d'usage de XpePerience.
//!
//! Chaque fonction publique vérifie les droits de l'utilisateur, valide les
//! données, écrit en base dans une transaction et trace l'historique. Les
//! permissions sont donc toujours appliquées côté serveur, quel que soit
//! l'appelant (interface ou extension).

pub mod admin;
pub mod auth;
pub mod crm;
pub mod linkedin;
pub mod needs;
pub mod overview;
pub mod positionings;
pub mod projects;
pub mod resources;

mod support;

pub use auth::AuthUser;
pub use xpeperience_store::Connection;
