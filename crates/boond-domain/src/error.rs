use serde::Serialize;

/// Erreur rattachée à un champ de formulaire.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field: field.into(), message: message.into() }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Certaines informations sont invalides.")]
    Validation(Vec<FieldError>),
    #[error("{0} introuvable.")]
    NotFound(&'static str),
    #[error("Vous n'avez pas les droits nécessaires pour cette action.")]
    Forbidden,
    #[error("Connexion requise.")]
    Unauthorized,
    #[error("{message}")]
    Conflict { message: String, details: Option<serde_json::Value> },
    /// Règle métier non respectée, avec un message lisible par l'utilisateur.
    #[error("{0}")]
    Rule(String),
    #[error("Erreur interne : {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, DomainError>;

impl DomainError {
    pub fn invalid(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation(vec![FieldError::new(field, message)])
    }

    pub fn conflict(message: impl Into<String>, details: Option<serde_json::Value>) -> Self {
        Self::Conflict { message: message.into(), details }
    }

    pub fn rule(message: impl Into<String>) -> Self {
        Self::Rule(message.into())
    }
}

/// Accumule les erreurs de champ pour les renvoyer en une seule fois.
#[derive(Debug, Default)]
pub struct Errors(Vec<FieldError>);

impl Errors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.0.push(FieldError::new(field, message));
    }

    pub fn push(&mut self, error: Option<FieldError>) {
        if let Some(e) = error {
            self.0.push(e);
        }
    }

    pub fn finish(self) -> Result<()> {
        if self.0.is_empty() { Ok(()) } else { Err(DomainError::Validation(self.0)) }
    }
}
