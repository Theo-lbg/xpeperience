//! Historique des créations, modifications et changements d'état.

use xpeperience_domain::Id;
use xpeperience_domain::model::{EntityKind, FieldChange, HistoryEntry};
use rusqlite::{Connection, Row, params};

use crate::sql::{named, query};
use crate::{Result, now};

pub struct NewEntry<'a> {
    pub entity: EntityKind,
    pub entity_id: Id,
    pub entity_label: &'a str,
    pub action: &'a str,
    pub changes: &'a [FieldChange],
    pub link: Option<(EntityKind, Id)>,
    pub user_id: Id,
}

pub fn record(conn: &Connection, e: &NewEntry) -> Result<()> {
    conn.execute(
        "INSERT INTO history (entity, entity_id, entity_label, action, changes, link_entity, link_id, user_id, at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            e.entity.code(),
            e.entity_id,
            e.entity_label,
            e.action,
            serde_json::to_string(e.changes)?,
            e.link.map(|(k, _)| k.code()),
            e.link.map(|(_, id)| id),
            e.user_id,
            now()
        ],
    )?;
    Ok(())
}

const SELECT: &str = "SELECT h.id, h.entity, h.entity_id, h.entity_label, h.action, h.changes, h.link_entity, h.link_id, \
     h.user_id, u.first_name || ' ' || u.last_name, h.at FROM history h LEFT JOIN users u ON u.id = h.user_id";

fn map(r: &Row) -> rusqlite::Result<HistoryEntry> {
    let entity: String = r.get(1)?;
    let changes: String = r.get(5)?;
    let link: Option<String> = r.get(6)?;
    Ok(HistoryEntry {
        id: r.get(0)?,
        entity: EntityKind::from_code(&entity).unwrap_or(EntityKind::Company),
        entity_id: r.get(2)?,
        entity_label: r.get(3)?,
        action: r.get(4)?,
        changes: serde_json::from_str(&changes).unwrap_or_default(),
        link_entity: link.as_deref().and_then(EntityKind::from_code),
        link_id: r.get(7)?,
        user: named(r.get(8)?, r.get(9)?),
        at: r.get(10)?,
    })
}

/// Historique d'un objet, y compris les entrées des objets qui lui sont rattachés
/// (ex. positionnements d'un besoin, prestations d'un projet).
pub fn for_entity(conn: &Connection, entity: EntityKind, id: Id) -> Result<Vec<HistoryEntry>> {
    query(
        conn,
        &format!(
            "{SELECT} WHERE (h.entity = ?1 AND h.entity_id = ?2) OR (h.link_entity = ?1 AND h.link_id = ?2) \
             ORDER BY h.at DESC, h.id DESC LIMIT 200"
        ),
        &[entity.code().to_string().into(), id.into()],
        map,
    )
}

/// Dernières actions sur les objets visibles depuis le tableau de bord.
pub fn recent(conn: &Connection, entities: &[EntityKind], limit: i64) -> Result<Vec<HistoryEntry>> {
    if entities.is_empty() {
        return Ok(Vec::new());
    }
    let list = entities.iter().map(|e| format!("'{}'", e.code())).collect::<Vec<_>>().join(", ");
    query(
        conn,
        &format!("{SELECT} WHERE h.entity IN ({list}) AND h.action <> 'deleted' ORDER BY h.at DESC, h.id DESC LIMIT ?1"),
        &[limit.into()],
        map,
    )
}
