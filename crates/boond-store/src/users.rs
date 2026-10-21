//! Utilisateurs, rôles, sessions et jetons d'appairage.

use boond_domain::Id;
use boond_domain::model::{ApiToken, Preferences, Role, User};
use boond_domain::permissions::Permissions;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::query;
use crate::{Result, now};

const USER_SELECT: &str = "SELECT u.id, u.email, u.first_name, u.last_name, u.role_id, r.name, r.is_admin, u.active, \
     u.created_at, u.last_login_at FROM users u JOIN roles r ON r.id = u.role_id";

fn map_user(r: &Row) -> rusqlite::Result<User> {
    Ok(User {
        id: r.get(0)?,
        email: r.get(1)?,
        first_name: r.get(2)?,
        last_name: r.get(3)?,
        role_id: r.get(4)?,
        role_name: r.get(5)?,
        is_admin: r.get(6)?,
        active: r.get(7)?,
        created_at: r.get(8)?,
        last_login_at: r.get(9)?,
    })
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?)
}

pub fn list(conn: &Connection) -> Result<Vec<User>> {
    query(conn, &format!("{USER_SELECT} ORDER BY u.last_name, u.first_name"), &[], map_user)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<User>> {
    Ok(conn.query_row(&format!("{USER_SELECT} WHERE u.id = ?1"), [id], map_user).optional()?)
}

/// Utilisateur et empreinte de mot de passe, pour la connexion.
pub fn find_for_login(conn: &Connection, email: &str) -> Result<Option<(User, String)>> {
    Ok(conn
        .query_row(
            "SELECT u.id, u.email, u.first_name, u.last_name, u.role_id, r.name, r.is_admin, u.active, \
             u.created_at, u.last_login_at, u.password_hash FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE u.email = ?1",
            [email],
            |r| Ok((map_user(r)?, r.get(10)?)),
        )
        .optional()?)
}

pub fn password_hash(conn: &Connection, id: Id) -> Result<Option<String>> {
    Ok(conn.query_row("SELECT password_hash FROM users WHERE id = ?1", [id], |r| r.get(0)).optional()?)
}

pub struct NewUser<'a> {
    pub email: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub password_hash: &'a str,
    pub role_id: Id,
    pub active: bool,
}

pub fn insert(conn: &Connection, u: &NewUser) -> Result<Id> {
    conn.execute(
        "INSERT INTO users (email, first_name, last_name, password_hash, role_id, active, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![u.email, u.first_name, u.last_name, u.password_hash, u.role_id, u.active, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, email: &str, first: &str, last: &str, role_id: Id, active: bool) -> Result<()> {
    conn.execute(
        "UPDATE users SET email = ?2, first_name = ?3, last_name = ?4, role_id = ?5, active = ?6 WHERE id = ?1",
        params![id, email, first, last, role_id, active],
    )?;
    if !active {
        conn.execute("DELETE FROM sessions WHERE user_id = ?1", [id])?;
    }
    Ok(())
}

pub fn set_password(conn: &Connection, id: Id, hash: &str) -> Result<()> {
    conn.execute("UPDATE users SET password_hash = ?2 WHERE id = ?1", params![id, hash])?;
    Ok(())
}

pub fn touch_login(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("UPDATE users SET last_login_at = ?2 WHERE id = ?1", params![id, now()])?;
    Ok(())
}

pub fn preferences(conn: &Connection, id: Id) -> Result<Preferences> {
    let raw: Option<String> =
        conn.query_row("SELECT preferences FROM users WHERE id = ?1", [id], |r| r.get(0)).optional()?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

pub fn set_preferences(conn: &Connection, id: Id, prefs: &Preferences) -> Result<()> {
    conn.execute("UPDATE users SET preferences = ?2 WHERE id = ?1", params![id, serde_json::to_string(prefs)?])?;
    Ok(())
}

/// Nombre d'administrateurs actifs (protection contre la perte d'accès).
pub fn active_admins(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM users u JOIN roles r ON r.id = u.role_id WHERE u.active = 1 AND r.is_admin = 1",
        [],
        |r| r.get(0),
    )?)
}

/// Liste courte des utilisateurs actifs, pour les sélecteurs de responsable.
pub fn directory(conn: &Connection) -> Result<Vec<boond_domain::model::Named>> {
    query(
        conn,
        "SELECT id, first_name || ' ' || last_name FROM users WHERE active = 1 ORDER BY first_name, last_name",
        &[],
        |r| Ok(boond_domain::model::Named { id: r.get(0)?, name: r.get(1)? }),
    )
}

// ─────────────────────────────── Rôles ───────────────────────────────

fn map_role(r: &Row) -> rusqlite::Result<Role> {
    let raw: String = r.get(4)?;
    Ok(Role {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        is_admin: r.get(3)?,
        permissions: serde_json::from_str(&raw).unwrap_or_default(),
        users_count: r.get(5)?,
    })
}

const ROLE_SELECT: &str = "SELECT r.id, r.name, r.description, r.is_admin, r.permissions, \
     (SELECT COUNT(*) FROM users u WHERE u.role_id = r.id) FROM roles r";

pub fn roles(conn: &Connection) -> Result<Vec<Role>> {
    query(conn, &format!("{ROLE_SELECT} ORDER BY r.is_admin DESC, r.name"), &[], map_role)
}

pub fn role(conn: &Connection, id: Id) -> Result<Option<Role>> {
    Ok(conn.query_row(&format!("{ROLE_SELECT} WHERE r.id = ?1"), [id], map_role).optional()?)
}

pub fn admin_role_id(conn: &Connection) -> Result<Id> {
    Ok(conn.query_row("SELECT id FROM roles WHERE is_admin = 1 ORDER BY id LIMIT 1", [], |r| r.get(0))?)
}

pub fn insert_role(conn: &Connection, name: &str, description: Option<&str>, perms: &Permissions) -> Result<Id> {
    conn.execute(
        "INSERT INTO roles (name, description, permissions, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![name, description, serde_json::to_string(perms)?, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_role(conn: &Connection, id: Id, name: &str, description: Option<&str>, perms: &Permissions) -> Result<()> {
    conn.execute(
        "UPDATE roles SET name = ?2, description = ?3, permissions = ?4 WHERE id = ?1",
        params![id, name, description, serde_json::to_string(perms)?],
    )?;
    Ok(())
}

pub fn delete_role(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM roles WHERE id = ?1", [id])?;
    Ok(())
}

// ────────────────────────────── Sessions ──────────────────────────────

pub fn create_session(conn: &Connection, token_hash: &str, user_id: Id, expires_at: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (token_hash, user_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![token_hash, user_id, now(), expires_at],
    )?;
    Ok(())
}

/// Utilisateur actif d'une session non expirée, avec l'échéance de la session.
pub fn session_user(conn: &Connection, token_hash: &str, now: &str) -> Result<Option<(Id, String)>> {
    Ok(conn
        .query_row(
            "SELECT s.user_id, s.expires_at FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token_hash = ?1 AND s.expires_at > ?2 AND u.active = 1",
            params![token_hash, now],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

pub fn extend_session(conn: &Connection, token_hash: &str, expires_at: &str) -> Result<()> {
    conn.execute("UPDATE sessions SET expires_at = ?2 WHERE token_hash = ?1", params![token_hash, expires_at])?;
    Ok(())
}

pub fn delete_session(conn: &Connection, token_hash: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE token_hash = ?1", [token_hash])?;
    Ok(())
}

pub fn delete_user_sessions(conn: &Connection, user_id: Id) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE user_id = ?1", [user_id])?;
    Ok(())
}

pub fn purge_sessions(conn: &Connection, now: &str) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", [now])?;
    Ok(())
}

// ─────────────────────── Jetons d'appairage (extension) ───────────────────────

const TOKEN_SELECT: &str = "SELECT t.id, t.name, t.prefix, t.user_id, u.first_name || ' ' || u.last_name, \
     t.created_at, t.last_used_at, t.revoked_at IS NOT NULL FROM api_tokens t JOIN users u ON u.id = t.user_id";

fn map_token(r: &Row) -> rusqlite::Result<ApiToken> {
    Ok(ApiToken {
        id: r.get(0)?,
        name: r.get(1)?,
        prefix: r.get(2)?,
        user_id: r.get(3)?,
        user_name: r.get(4)?,
        created_at: r.get(5)?,
        last_used_at: r.get(6)?,
        revoked: r.get(7)?,
    })
}

pub fn tokens(conn: &Connection, user_id: Option<Id>) -> Result<Vec<ApiToken>> {
    match user_id {
        Some(uid) => query(conn, &format!("{TOKEN_SELECT} WHERE t.user_id = ?1 ORDER BY t.id DESC"), &[uid.into()], map_token),
        None => query(conn, &format!("{TOKEN_SELECT} ORDER BY t.id DESC"), &[], map_token),
    }
}

pub fn token(conn: &Connection, id: Id) -> Result<Option<ApiToken>> {
    Ok(conn.query_row(&format!("{TOKEN_SELECT} WHERE t.id = ?1"), [id], map_token).optional()?)
}

pub fn insert_token(conn: &Connection, user_id: Id, name: &str, prefix: &str, hash: &str) -> Result<Id> {
    conn.execute(
        "INSERT INTO api_tokens (user_id, name, prefix, token_hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![user_id, name, prefix, hash, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn revoke_token(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("UPDATE api_tokens SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, now()])?;
    Ok(())
}

/// Utilisateur actif associé à un jeton valide ; met à jour la date de dernière utilisation.
pub fn token_user(conn: &Connection, hash: &str) -> Result<Option<Id>> {
    let user: Option<Id> = conn
        .query_row(
            "SELECT t.user_id FROM api_tokens t JOIN users u ON u.id = t.user_id \
             WHERE t.token_hash = ?1 AND t.revoked_at IS NULL AND u.active = 1",
            [hash],
            |r| r.get(0),
        )
        .optional()?;
    if user.is_some() {
        conn.execute("UPDATE api_tokens SET last_used_at = ?2 WHERE token_hash = ?1", params![hash, now()])?;
    }
    Ok(user)
}
