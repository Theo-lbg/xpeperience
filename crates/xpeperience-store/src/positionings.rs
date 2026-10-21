//! Positionnements : une ressource proposée sur un besoin.

use xpeperience_domain::Id;
use xpeperience_domain::model::{Named, Positioning, PositioningFilter, PositioningInput, ResourceKind};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{Filter, named, query};
use crate::{Result, now};

const SELECT: &str = "SELECT x.id, x.need_id, n.title, n.company_id, co.name, x.resource_id, \
     r.first_name || ' ' || r.last_name, r.kind, r.title, x.state_id, x.start_date, x.end_date, x.daily_rate, x.days, \
     x.daily_cost, x.notes, x.project_id, p.title, x.created_at, x.updated_at \
     FROM positionings x \
     JOIN needs n ON n.id = x.need_id \
     JOIN resources r ON r.id = x.resource_id \
     LEFT JOIN companies co ON co.id = n.company_id \
     LEFT JOIN projects p ON p.id = x.project_id \
     LEFT JOIN ref_items s ON s.id = x.state_id";

fn map(r: &Row) -> rusqlite::Result<Positioning> {
    let kind: String = r.get(7)?;
    Ok(Positioning {
        id: r.get(0)?,
        need: Named { id: r.get(1)?, name: r.get(2)? },
        company: named(r.get(3)?, r.get(4)?),
        resource: Named { id: r.get(5)?, name: r.get(6)? },
        resource_kind: ResourceKind::from_code(&kind).unwrap_or(ResourceKind::Candidate),
        resource_title: r.get(8)?,
        state_id: r.get(9)?,
        start_date: r.get(10)?,
        end_date: r.get(11)?,
        daily_rate: r.get(12)?,
        days: r.get(13)?,
        daily_cost: r.get(14)?,
        notes: r.get(15)?,
        project: named(r.get(16)?, r.get(17)?),
        created_at: r.get(18)?,
        updated_at: r.get(19)?,
    })
}

pub fn list(conn: &Connection, f: &PositioningFilter) -> Result<Vec<Positioning>> {
    let mut w = Filter::new();
    if let Some(v) = f.need_id {
        w.add("x.need_id = ?", v);
    }
    if let Some(v) = f.resource_id {
        w.add("x.resource_id = ?", v);
    }
    if let Some(c) = &f.state_category {
        w.add("s.category = ?", c.clone());
    }
    query(conn, &format!("{SELECT}{} ORDER BY s.position, x.updated_at DESC", w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Positioning>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE x.id = ?1"), [id], map).optional()?)
}

pub fn exists(conn: &Connection, need_id: Id, resource_id: Id) -> Result<Option<Id>> {
    Ok(conn
        .query_row(
            "SELECT id FROM positionings WHERE need_id = ?1 AND resource_id = ?2",
            params![need_id, resource_id],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn insert(conn: &Connection, need_id: Id, resource_id: Id, state_id: Id, i: &PositioningInput, user_id: Id) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO positionings (need_id, resource_id, state_id, start_date, end_date, daily_rate, days, daily_cost, \
         notes, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![need_id, resource_id, state_id, i.start_date, i.end_date, i.daily_rate, i.days, i.daily_cost, i.notes, user_id, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, state_id: Id, i: &PositioningInput) -> Result<()> {
    conn.execute(
        "UPDATE positionings SET state_id = ?2, start_date = ?3, end_date = ?4, daily_rate = ?5, days = ?6, \
         daily_cost = ?7, notes = ?8, updated_at = ?9 WHERE id = ?1",
        params![id, state_id, i.start_date, i.end_date, i.daily_rate, i.days, i.daily_cost, i.notes, now()],
    )?;
    Ok(())
}

pub fn set_project(conn: &Connection, id: Id, project_id: Id) -> Result<()> {
    conn.execute("UPDATE positionings SET project_id = ?2, updated_at = ?3 WHERE id = ?1", params![id, project_id, now()])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM positionings WHERE id = ?1", [id])?;
    Ok(())
}

/// Identifiants des ressources déjà positionnées sur un besoin.
pub fn resource_ids_for_need(conn: &Connection, need_id: Id) -> Result<Vec<Id>> {
    query(conn, "SELECT resource_id FROM positionings WHERE need_id = ?1", &[need_id.into()], |r| r.get(0))
}
