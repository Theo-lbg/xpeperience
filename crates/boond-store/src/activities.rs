//! Activités commerciales : appels, rendez-vous, e-mails, relances, notes.

use boond_domain::Id;
use boond_domain::model::{Activity, ActivityFilter, ActivityKind};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{Filter, named, person, query};
use crate::{Result, now};

const SELECT: &str = "SELECT a.id, a.kind, a.occurred_at, a.comment, a.due_date, a.done_at, a.company_id, co.name, \
     a.contact_id, ct.first_name, ct.last_name, a.need_id, n.title, a.resource_id, r.first_name || ' ' || r.last_name, \
     a.positioning_id, a.project_id, p.title, a.owner_id, u.first_name || ' ' || u.last_name, a.created_at \
     FROM activities a \
     LEFT JOIN companies co ON co.id = a.company_id \
     LEFT JOIN contacts ct ON ct.id = a.contact_id \
     LEFT JOIN needs n ON n.id = a.need_id \
     LEFT JOIN resources r ON r.id = a.resource_id \
     LEFT JOIN projects p ON p.id = a.project_id \
     LEFT JOIN users u ON u.id = a.owner_id";

fn map(r: &Row) -> rusqlite::Result<Activity> {
    let kind: String = r.get(1)?;
    Ok(Activity {
        id: r.get(0)?,
        kind: ActivityKind::from_code(&kind).unwrap_or(ActivityKind::Note),
        occurred_at: r.get(2)?,
        comment: r.get(3)?,
        due_date: r.get(4)?,
        done_at: r.get(5)?,
        company: named(r.get(6)?, r.get(7)?),
        contact: named(r.get(8)?, person(r.get(9)?, r.get(10)?)),
        need: named(r.get(11)?, r.get(12)?),
        resource: named(r.get(13)?, r.get(14)?),
        positioning_id: r.get(15)?,
        project: named(r.get(16)?, r.get(17)?),
        owner: named(r.get(18)?, r.get(19)?),
        created_at: r.get(20)?,
    })
}

pub fn list(conn: &Connection, f: &ActivityFilter) -> Result<Vec<Activity>> {
    let mut w = Filter::new();
    for (column, value) in [
        ("a.company_id", f.company_id),
        ("a.contact_id", f.contact_id),
        ("a.need_id", f.need_id),
        ("a.resource_id", f.resource_id),
        ("a.positioning_id", f.positioning_id),
        ("a.project_id", f.project_id),
        ("a.owner_id", f.owner_id),
    ] {
        if let Some(v) = value {
            w.add(&format!("{column} = ?"), v);
        }
    }
    let reminders = f.reminders.unwrap_or(false);
    if reminders {
        w.raw("a.due_date IS NOT NULL AND a.done_at IS NULL");
    }
    if let Some(d) = &f.due_before {
        w.add("a.due_date <= ?", d.clone());
    }
    let order = if reminders { "a.due_date, a.id" } else { "a.occurred_at DESC, a.id DESC" };
    let limit = f.limit.unwrap_or(200).clamp(1, 500);
    query(conn, &format!("{SELECT}{} ORDER BY {order} LIMIT {limit}", w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Activity>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE a.id = ?1"), [id], map).optional()?)
}

/// Activité prête à être enregistrée (liens déjà complétés par le service).
pub struct NewActivity<'a> {
    pub kind: ActivityKind,
    pub occurred_at: &'a str,
    pub comment: &'a str,
    pub due_date: Option<&'a str>,
    pub company_id: Option<Id>,
    pub contact_id: Option<Id>,
    pub need_id: Option<Id>,
    pub resource_id: Option<Id>,
    pub positioning_id: Option<Id>,
    pub project_id: Option<Id>,
    pub owner_id: Option<Id>,
}

pub fn insert(conn: &Connection, a: &NewActivity, user_id: Id) -> Result<Id> {
    conn.execute(
        "INSERT INTO activities (kind, occurred_at, comment, due_date, company_id, contact_id, need_id, resource_id, \
         positioning_id, project_id, owner_id, created_by, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            a.kind.code(), a.occurred_at, a.comment, a.due_date, a.company_id, a.contact_id, a.need_id, a.resource_id,
            a.positioning_id, a.project_id, a.owner_id, user_id, now()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, a: &NewActivity) -> Result<()> {
    conn.execute(
        "UPDATE activities SET kind = ?2, occurred_at = ?3, comment = ?4, due_date = ?5, owner_id = ?6 WHERE id = ?1",
        params![id, a.kind.code(), a.occurred_at, a.comment, a.due_date, a.owner_id],
    )?;
    Ok(())
}

pub fn set_done(conn: &Connection, id: Id, done: bool) -> Result<()> {
    let value = done.then(now);
    conn.execute("UPDATE activities SET done_at = ?2 WHERE id = ?1", params![id, value])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM activities WHERE id = ?1", [id])?;
    Ok(())
}
