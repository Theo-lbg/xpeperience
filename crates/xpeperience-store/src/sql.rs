//! Petits outils partagés par les requêtes.

use xpeperience_domain::Id;
use xpeperience_domain::model::Named;
use rusqlite::types::Value;
use rusqlite::{Connection, Row};

use crate::Result;

/// Clause WHERE construite au fil des filtres, avec ses paramètres positionnels.
#[derive(Default)]
pub(crate) struct Filter {
    clauses: Vec<String>,
    pub params: Vec<Value>,
}

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute une condition contenant un seul `?`.
    pub fn add(&mut self, clause: &str, value: impl Into<Value>) {
        self.clauses.push(clause.to_string());
        self.params.push(value.into());
    }

    /// Ajoute une condition contenant plusieurs `?`, dans l'ordre des valeurs.
    pub fn add_many(&mut self, clause: &str, values: Vec<Value>) {
        self.clauses.push(clause.to_string());
        self.params.extend(values);
    }

    pub fn raw(&mut self, clause: &str) {
        self.clauses.push(clause.to_string());
    }

    /// Ajoute une recherche plein texte sur une expression SQL.
    pub fn search(&mut self, expr: &str, q: Option<&str>) {
        if let Some(q) = q.map(str::trim).filter(|q| !q.is_empty()) {
            self.add(&format!("fold({expr}) LIKE ? ESCAPE '\\'"), xpeperience_domain::text::like_pattern(q));
        }
    }

    /// Exclut les objets archivés, sauf demande contraire.
    pub fn archived(&mut self, alias: &str, archived: Option<bool>) {
        if archived.unwrap_or(false) {
            self.raw(&format!("{alias}.archived_at IS NOT NULL"));
        } else {
            self.raw(&format!("{alias}.archived_at IS NULL"));
        }
    }

    pub fn sql(&self) -> String {
        if self.clauses.is_empty() { String::new() } else { format!(" WHERE {}", self.clauses.join(" AND ")) }
    }
}

pub(crate) fn query<T>(
    conn: &Connection,
    sql: &str,
    params: &[Value],
    map: impl FnMut(&Row) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), map)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(crate) fn named(id: Option<Id>, name: Option<String>) -> Option<Named> {
    match (id, name) {
        (Some(id), Some(name)) => Some(Named { id, name }),
        (Some(id), None) => Some(Named { id, name: String::new() }),
        _ => None,
    }
}

pub(crate) fn person(first: Option<String>, last: Option<String>) -> Option<String> {
    last.map(|l| xpeperience_domain::model::person_name(first.as_deref(), &l))
}

pub(crate) fn tags_to_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".into())
}

pub(crate) fn tags_from_json(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Expression SQL : fin effective d'une prestation `d` (prolongations signées incluses).
/// Une prestation sans fin est considérée comme ouverte (`9999-12-31`).
pub(crate) const DELIVERY_EFFECTIVE_END: &str = "CASE WHEN d.end_date IS NULL THEN '9999-12-31' \
     ELSE MAX(d.end_date, COALESCE((SELECT MAX(e.end_date) FROM delivery_extensions e \
     WHERE e.delivery_id = d.id AND e.status = 'signed'), d.end_date)) END";

/// Vérifie qu'une date générée par l'application peut être insérée telle quelle dans le SQL.
pub(crate) fn iso_literal(date: &str) -> String {
    if date.len() == 10 && date.chars().all(|c| c.is_ascii_digit() || c == '-') { date.to_string() } else { "0000-00-00".into() }
}
