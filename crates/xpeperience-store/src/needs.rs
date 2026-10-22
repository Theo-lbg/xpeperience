//! Besoins clients (opportunités).

use xpeperience_domain::Id;
use xpeperience_domain::model::{Need, NeedFilter, NeedInput};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{Filter, named, person, query, tags_from_json, tags_to_json};
use crate::{Result, now};

const SELECT: &str = "SELECT n.id, n.title, n.company_id, co.name, n.contact_id, ct.first_name, ct.last_name, \
     n.description, n.skills, n.type_id, n.state_id, n.origin_id, n.budget, n.duration_months, n.planned_days, \
     n.start_date, n.location, n.owner_id, u.first_name || ' ' || u.last_name, n.project_id, p.title, \
     n.archived_at IS NOT NULL, n.created_at, n.updated_at, \
     (SELECT COUNT(*) FROM positionings x WHERE x.need_id = n.id), n.tender_reference \
     FROM needs n \
     LEFT JOIN companies co ON co.id = n.company_id \
     LEFT JOIN contacts ct ON ct.id = n.contact_id \
     LEFT JOIN users u ON u.id = n.owner_id \
     LEFT JOIN projects p ON p.id = n.project_id \
     LEFT JOIN ref_items s ON s.id = n.state_id";

fn map(r: &Row) -> rusqlite::Result<Need> {
    Ok(Need {
        id: r.get(0)?,
        title: r.get(1)?,
        company: named(r.get(2)?, r.get(3)?),
        contact: named(r.get(4)?, person(r.get(5)?, r.get(6)?)),
        description: r.get(7)?,
        skills: tags_from_json(r.get(8)?),
        type_id: r.get(9)?,
        state_id: r.get(10)?,
        origin_id: r.get(11)?,
        budget: r.get(12)?,
        duration_months: r.get(13)?,
        planned_days: r.get(14)?,
        start_date: r.get(15)?,
        location: r.get(16)?,
        owner: named(r.get(17)?, r.get(18)?),
        project: named(r.get(19)?, r.get(20)?),
        archived: r.get(21)?,
        created_at: r.get(22)?,
        updated_at: r.get(23)?,
        positionings_count: r.get(24)?,
        tender_reference: r.get(25)?,
    })
}

pub fn list(conn: &Connection, f: &NeedFilter) -> Result<Vec<Need>> {
    let mut w = Filter::new();
    w.archived("n", f.archived);
    w.search(
        "n.title || ' ' || COALESCE(n.description, '') || ' ' || n.skills || ' ' || COALESCE(n.tender_reference, '') || ' ' || COALESCE(co.name, '') || ' ' || \
         COALESCE(ct.first_name, '') || ' ' || ct.last_name",
        f.q.as_deref(),
    );
    for (column, value) in [
        ("n.company_id", f.company_id),
        ("n.contact_id", f.contact_id),
        ("n.owner_id", f.owner_id),
        ("n.state_id", f.state_id),
        ("n.type_id", f.type_id),
    ] {
        if let Some(v) = value {
            w.add(&format!("{column} = ?"), v);
        }
    }
    if let Some(c) = &f.state_category {
        w.add("s.category = ?", c.clone());
    }
    if let Some(d) = &f.start_from {
        w.add("n.start_date >= ?", d.clone());
    }
    if let Some(d) = &f.start_to {
        w.add("n.start_date <= ?", d.clone());
    }
    query(conn, &format!("{SELECT}{} ORDER BY n.updated_at DESC LIMIT 1000", w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Need>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE n.id = ?1"), [id], map).optional()?)
}

pub fn insert(conn: &Connection, i: &NeedInput, company_id: Option<Id>, state_id: Id, user_id: Id) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO needs (title, company_id, contact_id, description, skills, type_id, state_id, origin_id, budget, \
         duration_months, planned_days, start_date, location, owner_id, created_by, created_at, updated_at, tender_reference) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16, ?17)",
        params![
            i.title, company_id, i.contact_id, i.description, tags_to_json(&i.skills), i.type_id, state_id, i.origin_id,
            i.budget, i.duration_months, i.planned_days, i.start_date, i.location, i.owner_id, user_id, now, i.tender_reference
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, i: &NeedInput, company_id: Option<Id>, state_id: Id) -> Result<()> {
    conn.execute(
        "UPDATE needs SET title = ?2, company_id = ?3, contact_id = ?4, description = ?5, skills = ?6, type_id = ?7, \
         state_id = ?8, origin_id = ?9, budget = ?10, duration_months = ?11, planned_days = ?12, start_date = ?13, \
         location = ?14, owner_id = ?15, updated_at = ?16, tender_reference = ?17 WHERE id = ?1",
        params![
            id, i.title, company_id, i.contact_id, i.description, tags_to_json(&i.skills), i.type_id, state_id,
            i.origin_id, i.budget, i.duration_months, i.planned_days, i.start_date, i.location, i.owner_id, now(), i.tender_reference
        ],
    )?;
    Ok(())
}

pub fn set_state(conn: &Connection, id: Id, state_id: Id) -> Result<()> {
    conn.execute("UPDATE needs SET state_id = ?2, updated_at = ?3 WHERE id = ?1", params![id, state_id, now()])?;
    Ok(())
}

pub fn set_project(conn: &Connection, id: Id, project_id: Id) -> Result<()> {
    conn.execute("UPDATE needs SET project_id = ?2, updated_at = ?3 WHERE id = ?1", params![id, project_id, now()])?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: Id, archived: bool) -> Result<()> {
    let now = now();
    let value = archived.then_some(now.clone());
    conn.execute("UPDATE needs SET archived_at = ?2, updated_at = ?3 WHERE id = ?1", params![id, value, now])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM needs WHERE id = ?1", [id])?;
    Ok(())
}

/// Met à jour la société des besoins d'un contact qui change de société.
pub fn sync_company_for_contact(conn: &Connection, contact_id: Id, company_id: Option<Id>) -> Result<()> {
    conn.execute(
        "UPDATE needs SET company_id = ?2 WHERE contact_id = ?1 AND company_id IS NOT ?2",
        params![contact_id, company_id],
    )?;
    Ok(())
}
