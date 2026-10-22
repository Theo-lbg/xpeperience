//! Listes paramétrables.

use xpeperience_domain::Id;
use xpeperience_domain::refs::{RefItem, RefList};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::Result;
use crate::sql::query;

const SELECT: &str = "SELECT id, list, label, category, color, position, active FROM ref_items";

fn map(r: &Row) -> rusqlite::Result<RefItem> {
    let list: String = r.get(1)?;
    Ok(RefItem {
        id: r.get(0)?,
        list: RefList::from_code(&list).unwrap_or(RefList::Sector),
        label: r.get(2)?,
        category: r.get(3)?,
        color: r.get(4)?,
        position: r.get(5)?,
        active: r.get(6)?,
    })
}

pub fn all(conn: &Connection) -> Result<Vec<RefItem>> {
    query(conn, &format!("{SELECT} ORDER BY list, position, id"), &[], map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<RefItem>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], map).optional()?)
}

/// Premier élément actif d'une liste, éventuellement limité à une catégorie.
pub fn first(conn: &Connection, list: RefList, category: Option<&str>) -> Result<Option<RefItem>> {
    Ok(conn
        .query_row(
            &format!("{SELECT} WHERE list = ?1 AND active = 1 AND (?2 IS NULL OR category = ?2) ORDER BY position, id LIMIT 1"),
            params![list.code(), category],
            map,
        )
        .optional()?)
}

pub fn insert(conn: &Connection, list: RefList, label: &str, category: Option<&str>, color: Option<&str>) -> Result<Id> {
    let pos: i64 =
        conn.query_row("SELECT COALESCE(MAX(position), 0) + 1 FROM ref_items WHERE list = ?1", [list.code()], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO ref_items (list, label, category, color, position) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![list.code(), label, category, color, pos],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, label: &str, category: Option<&str>, color: Option<&str>, active: bool) -> Result<()> {
    conn.execute(
        "UPDATE ref_items SET label = ?2, category = ?3, color = ?4, active = ?5 WHERE id = ?1",
        params![id, label, category, color, active],
    )?;
    Ok(())
}

pub fn reorder(conn: &Connection, list: RefList, ids: &[Id]) -> Result<()> {
    for (pos, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE ref_items SET position = ?3 WHERE id = ?1 AND list = ?2",
            params![id, list.code(), pos as i64 + 1],
        )?;
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM ref_items WHERE id = ?1", [id])?;
    Ok(())
}

/// Nombre d'objets qui utilisent cet élément, toutes tables confondues.
pub fn usage(conn: &Connection, id: Id) -> Result<i64> {
    const USES: &[(&str, &str)] = &[
        ("companies", "type_id"),
        ("companies", "sector_id"),
        ("needs", "type_id"),
        ("needs", "state_id"),
        ("needs", "origin_id"),
        ("positionings", "state_id"),
        ("projects", "type_id"),
        ("projects", "state_id"),
        ("deliveries", "type_id"),
        ("resources", "contract_type_id"),
    ];
    let mut total = 0;
    for (table, column) in USES {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {column} = ?1"), [id], |r| r.get(0))?;
        total += n;
    }
    Ok(total)
}

/// Nombre d'éléments actifs d'une catégorie dans une liste.
pub fn count_active_in_category(conn: &Connection, list: RefList, category: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM ref_items WHERE list = ?1 AND category = ?2 AND active = 1",
        params![list.code(), category],
        |r| r.get(0),
    )?)
}
