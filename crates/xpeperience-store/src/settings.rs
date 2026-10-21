//! Réglages généraux, stockés en JSON sous une clé unique.

use xpeperience_domain::settings::Settings;
use rusqlite::{Connection, OptionalExtension, params};

use crate::Result;

const KEY: &str = "settings";

pub fn load(conn: &Connection) -> Result<Settings> {
    let raw: Option<String> = conn.query_row("SELECT value FROM settings WHERE key = ?1", [KEY], |r| r.get(0)).optional()?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![KEY, serde_json::to_string(settings)?],
    )?;
    Ok(())
}
