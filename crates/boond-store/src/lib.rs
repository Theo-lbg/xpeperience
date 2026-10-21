//! Persistance SQLite de Boond by XPEHO : schéma, migrations et requêtes.
//!
//! Ce crate ne décide d'aucune règle métier : les droits et validations sont
//! vérifiés par `boond-service`. Il peut appeler les fonctions de calcul pures
//! du domaine pour remplir les valeurs dérivées (fin effective d'une prestation…).

pub mod activities;
pub mod companies;
pub mod contacts;
pub mod dashboard;
pub mod history;
pub mod needs;
pub mod positionings;
pub mod projects;
pub mod refs;
pub mod resources;
pub mod search;
pub mod settings;
pub mod users;

mod sql;

use std::path::Path;
use std::time::Duration;

use boond_domain::DomainError;
use rusqlite::functions::FunctionFlags;
pub use rusqlite::{Connection, Transaction};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

impl From<StoreError> for DomainError {
    fn from(e: StoreError) -> Self {
        if let StoreError::Sql(rusqlite::Error::SqliteFailure(f, msg)) = &e
            && f.code == rusqlite::ErrorCode::ConstraintViolation
        {
            let msg = msg.as_deref().unwrap_or_default();
            return if msg.contains("UNIQUE") {
                DomainError::conflict("Cet élément existe déjà.", None)
            } else if msg.contains("FOREIGN KEY") {
                DomainError::rule("Un élément lié est introuvable ou encore utilisé ailleurs.")
            } else {
                DomainError::rule("Les données ne respectent pas une contrainte de la base.")
            };
        }
        DomainError::Internal(e.to_string())
    }
}

/// Horodatage courant au format RFC 3339 (UTC, à la seconde).
pub fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Date du jour au format `AAAA-MM-JJ`, selon l'heure locale.
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Ouvre (ou crée) la base et applique les migrations.
pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    migrate(&conn)?;
    Ok(conn)
}

/// Copie cohérente de la base vers `dest`, même si le serveur écrit en même temps (`VACUUM INTO`).
/// Le fichier de destination ne doit pas exister.
pub fn backup_to(conn: &Connection, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Err(StoreError::Sql(rusqlite::Error::InvalidParameterName(format!("{} existe déjà", dest.display()))));
    }
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()])?;
    Ok(())
}

/// Base en mémoire, pour les tests.
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    // fold(texte) : minuscules sans accents, pour la recherche.
    conn.create_scalar_function("fold", 1, FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC, |ctx| {
        let value: Option<String> = ctx.get(0)?;
        Ok(value.map(|v| boond_domain::text::fold(&v)))
    })?;
    Ok(())
}

const MIGRATIONS: &[&str] = &[include_str!("schema.sql")];

fn migrate(conn: &Connection) -> Result<()> {
    let version: usize = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        if i == 0 {
            seed(&tx)?;
        }
        tx.pragma_update(None, "user_version", i + 1)?;
        tx.commit()?;
    }
    Ok(())
}

/// Paramètres initiaux : listes et rôles par défaut. Aucune donnée métier.
fn seed(conn: &Connection) -> Result<()> {
    let now = now();
    let mut positions = std::collections::HashMap::new();
    for (list, label, category, color) in boond_domain::refs::defaults() {
        let pos = positions.entry(list).or_insert(0i64);
        *pos += 1;
        conn.execute(
            "INSERT INTO ref_items (list, label, category, color, position) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![list.code(), label, category, color, *pos],
        )?;
    }
    for role in boond_domain::permissions::default_roles() {
        conn.execute(
            "INSERT INTO roles (name, description, is_admin, permissions, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![role.name, role.description, role.is_admin, serde_json::to_string(&role.permissions)?, now],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_seed_parameters_but_no_business_data() {
        let conn = open_in_memory().unwrap();
        let refs: i64 = conn.query_row("SELECT COUNT(*) FROM ref_items", [], |r| r.get(0)).unwrap();
        let roles: i64 = conn.query_row("SELECT COUNT(*) FROM roles", [], |r| r.get(0)).unwrap();
        let companies: i64 = conn.query_row("SELECT COUNT(*) FROM companies", [], |r| r.get(0)).unwrap();
        assert!(refs > 40);
        assert_eq!(roles, 4);
        assert_eq!(companies, 0);
        let folded: String = conn.query_row("SELECT fold('Société')", [], |r| r.get(0)).unwrap();
        assert_eq!(folded, "societe");
    }
}
