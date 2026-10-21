//! Modèle métier de XpePerience.
//!
//! Ce crate ne fait aucune entrée/sortie : il décrit les objets métier, leurs
//! validations, les permissions et les règles (dates de projet, CJM, matching).

pub mod error;
pub mod matching;
pub mod model;
pub mod permissions;
pub mod refs;
pub mod rules;
pub mod settings;
pub mod text;

pub use error::{DomainError, FieldError, Result};

/// Identifiant technique d'un objet métier.
pub type Id = i64;
