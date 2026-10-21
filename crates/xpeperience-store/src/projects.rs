//! Projets, prestations et prolongations.

use xpeperience_domain::Id;
use xpeperience_domain::model::{
    Delivery, DeliveryFilter, DeliveryInput, Extension, ExtensionInput, ExtensionStatus, Named, Project, ProjectFilter,
    ProjectInput,
};
use xpeperience_domain::rules::delivery_ends;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{Filter, named, person, query};
use crate::{Result, now};

const SELECT: &str = "SELECT p.id, p.reference, p.title, p.company_id, co.name, p.contact_id, ct.first_name, \
     ct.last_name, p.type_id, p.state_id, p.manager_id, u.first_name || ' ' || u.last_name, p.start_date, p.end_date, \
     p.forecast_end_date, (SELECT COUNT(*) FROM deliveries d WHERE d.project_id = p.id), p.need_id, n.title, p.notes, \
     p.archived_at IS NOT NULL, p.created_at, p.updated_at, \
     (SELECT json_group_array(json_object('id', rr.id, 'name', rr.name)) FROM \
        (SELECT DISTINCT r.id AS id, r.first_name || ' ' || r.last_name AS name FROM deliveries d \
         JOIN resources r ON r.id = d.resource_id WHERE d.project_id = p.id ORDER BY name) rr) \
     FROM projects p \
     LEFT JOIN companies co ON co.id = p.company_id \
     LEFT JOIN contacts ct ON ct.id = p.contact_id \
     LEFT JOIN users u ON u.id = p.manager_id \
     LEFT JOIN needs n ON n.id = p.need_id";

fn map(r: &Row) -> rusqlite::Result<Project> {
    let deliveries: i64 = r.get(15)?;
    let resources: Option<String> = r.get(22)?;
    Ok(Project {
        id: r.get(0)?,
        reference: r.get(1)?,
        title: r.get(2)?,
        company: named(r.get(3)?, r.get(4)?),
        contact: named(r.get(5)?, person(r.get(6)?, r.get(7)?)),
        type_id: r.get(8)?,
        state_id: r.get(9)?,
        manager: named(r.get(10)?, r.get(11)?),
        start_date: r.get(12)?,
        end_date: r.get(13)?,
        forecast_end_date: r.get(14)?,
        dates_from_deliveries: deliveries > 0,
        deliveries_count: deliveries,
        need: named(r.get(16)?, r.get(17)?),
        notes: r.get(18)?,
        archived: r.get(19)?,
        created_at: r.get(20)?,
        updated_at: r.get(21)?,
        resources: resources.and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default(),
    })
}

pub fn list(conn: &Connection, f: &ProjectFilter) -> Result<Vec<Project>> {
    let mut w = Filter::new();
    w.archived("p", f.archived);
    w.search(
        "p.title || ' ' || COALESCE(p.reference, '') || ' ' || COALESCE(co.name, '') || ' ' || COALESCE(p.notes, '')",
        f.q.as_deref(),
    );
    for (column, value) in [
        ("p.company_id", f.company_id),
        ("p.manager_id", f.manager_id),
        ("p.state_id", f.state_id),
        ("p.type_id", f.type_id),
    ] {
        if let Some(v) = value {
            w.add(&format!("{column} = ?"), v);
        }
    }
    if let Some(v) = f.resource_id {
        w.add("EXISTS (SELECT 1 FROM deliveries d WHERE d.project_id = p.id AND d.resource_id = ?)", v);
    }
    if let Some(d) = &f.period_to {
        w.add("(p.start_date IS NULL OR p.start_date <= ?)", d.clone());
    }
    if let Some(d) = &f.period_from {
        w.add("(COALESCE(p.forecast_end_date, p.end_date) IS NULL OR COALESCE(p.forecast_end_date, p.end_date) >= ?)", d.clone());
    }
    query(conn, &format!("{SELECT}{} ORDER BY p.start_date DESC NULLS LAST, p.id DESC LIMIT 1000", w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Project>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE p.id = ?1"), [id], map).optional()?)
}

pub fn insert(conn: &Connection, i: &ProjectInput, need_id: Option<Id>, user_id: Id) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO projects (reference, title, company_id, contact_id, type_id, state_id, manager_id, start_date, \
         end_date, forecast_end_date, need_id, notes, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            i.reference, i.title, i.company_id, i.contact_id, i.type_id, i.state_id, i.manager_id, i.start_date,
            i.end_date, need_id, i.notes, user_id, now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, i: &ProjectInput) -> Result<()> {
    conn.execute(
        "UPDATE projects SET reference = ?2, title = ?3, company_id = ?4, contact_id = ?5, type_id = ?6, state_id = ?7, \
         manager_id = ?8, notes = ?9, updated_at = ?10 WHERE id = ?1",
        params![id, i.reference, i.title, i.company_id, i.contact_id, i.type_id, i.state_id, i.manager_id, i.notes, now()],
    )?;
    Ok(())
}

pub fn set_dates(conn: &Connection, id: Id, start: Option<&str>, end: Option<&str>, forecast: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE projects SET start_date = ?2, end_date = ?3, forecast_end_date = ?4 WHERE id = ?1",
        params![id, start, end, forecast],
    )?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: Id, archived: bool) -> Result<()> {
    let now = now();
    let value = archived.then_some(now.clone());
    conn.execute("UPDATE projects SET archived_at = ?2, updated_at = ?3 WHERE id = ?1", params![id, value, now])?;
    Ok(())
}

pub fn touch(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("UPDATE projects SET updated_at = ?2 WHERE id = ?1", params![id, now()])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("UPDATE needs SET project_id = NULL WHERE project_id = ?1", [id])?;
    conn.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    Ok(())
}

/// Projets ouverts d'une société, pour rattacher une nouvelle prestation.
pub fn open_for_company(conn: &Connection, company_id: Id) -> Result<Vec<Named>> {
    query(
        conn,
        "SELECT p.id, p.title FROM projects p LEFT JOIN ref_items s ON s.id = p.state_id \
         WHERE p.company_id = ?1 AND p.archived_at IS NULL AND COALESCE(s.category, 'active') NOT IN ('done', 'cancelled') \
         ORDER BY p.updated_at DESC",
        &[company_id.into()],
        |r| Ok(Named { id: r.get(0)?, name: r.get(1)? }),
    )
}

// ───────────────────────────── Prestations ─────────────────────────────

const DELIVERY_SELECT: &str = "SELECT d.id, d.project_id, p.title, p.company_id, co.name, d.resource_id, \
     r.first_name || ' ' || r.last_name, d.type_id, d.title, d.start_date, d.end_date, d.daily_rate, d.sold_days, \
     d.daily_cost, d.notes, d.created_at, d.updated_at \
     FROM deliveries d JOIN projects p ON p.id = d.project_id \
     LEFT JOIN companies co ON co.id = p.company_id \
     LEFT JOIN resources r ON r.id = d.resource_id";

fn map_delivery(r: &Row) -> rusqlite::Result<Delivery> {
    Ok(Delivery {
        id: r.get(0)?,
        project: Named { id: r.get(1)?, name: r.get(2)? },
        company: named(r.get(3)?, r.get(4)?),
        resource: named(r.get(5)?, r.get(6)?),
        type_id: r.get(7)?,
        title: r.get(8)?,
        start_date: r.get(9)?,
        end_date: r.get(10)?,
        daily_rate: r.get(11)?,
        sold_days: r.get(12)?,
        daily_cost: r.get(13)?,
        notes: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        extensions: Vec::new(),
        effective_end_date: None,
        forecast_end_date: None,
    })
}

/// Complète une prestation avec ses prolongations et ses fins calculées.
fn with_extensions(conn: &Connection, mut d: Delivery) -> Result<Delivery> {
    d.extensions = extensions(conn, d.id)?;
    let spans: Vec<(ExtensionStatus, &str)> = d.extensions.iter().map(|e| (e.status, e.end_date.as_str())).collect();
    let (effective, forecast) = delivery_ends(d.end_date.as_deref(), &spans);
    d.effective_end_date = effective;
    d.forecast_end_date = forecast;
    Ok(d)
}

pub fn deliveries(conn: &Connection, f: &DeliveryFilter) -> Result<Vec<Delivery>> {
    let mut w = Filter::new();
    if let Some(v) = f.project_id {
        w.add("d.project_id = ?", v);
    }
    if let Some(v) = f.resource_id {
        w.add("d.resource_id = ?", v);
    }
    let rows = query(conn, &format!("{DELIVERY_SELECT}{} ORDER BY d.start_date, d.id", w.sql()), &w.params, map_delivery)?;
    rows.into_iter().map(|d| with_extensions(conn, d)).collect()
}

pub fn delivery(conn: &Connection, id: Id) -> Result<Option<Delivery>> {
    let d = conn.query_row(&format!("{DELIVERY_SELECT} WHERE d.id = ?1"), [id], map_delivery).optional()?;
    d.map(|d| with_extensions(conn, d)).transpose()
}

pub fn insert_delivery(
    conn: &Connection,
    project_id: Id,
    i: &DeliveryInput,
    start_date: &str,
    positioning_id: Option<Id>,
) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO deliveries (project_id, resource_id, type_id, title, start_date, end_date, daily_rate, sold_days, \
         daily_cost, notes, positioning_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            project_id, i.resource_id, i.type_id, i.title, start_date, i.end_date, i.daily_rate, i.sold_days, i.daily_cost,
            i.notes, positioning_id, now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_delivery(conn: &Connection, id: Id, i: &DeliveryInput, start_date: &str) -> Result<()> {
    conn.execute(
        "UPDATE deliveries SET resource_id = ?2, type_id = ?3, title = ?4, start_date = ?5, end_date = ?6, \
         daily_rate = ?7, sold_days = ?8, daily_cost = ?9, notes = ?10, updated_at = ?11 WHERE id = ?1",
        params![id, i.resource_id, i.type_id, i.title, start_date, i.end_date, i.daily_rate, i.sold_days, i.daily_cost, i.notes, now()],
    )?;
    Ok(())
}

pub fn delete_delivery(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM deliveries WHERE id = ?1", [id])?;
    Ok(())
}

// ──────────────────────────── Prolongations ────────────────────────────

const EXTENSION_SELECT: &str =
    "SELECT id, delivery_id, status, start_date, end_date, days, daily_rate, notes, created_at FROM delivery_extensions";

fn map_extension(r: &Row) -> rusqlite::Result<Extension> {
    let status: String = r.get(2)?;
    Ok(Extension {
        id: r.get(0)?,
        delivery_id: r.get(1)?,
        status: ExtensionStatus::from_code(&status).unwrap_or(ExtensionStatus::Forecast),
        start_date: r.get(3)?,
        end_date: r.get(4)?,
        days: r.get(5)?,
        daily_rate: r.get(6)?,
        notes: r.get(7)?,
        created_at: r.get(8)?,
    })
}

pub fn extensions(conn: &Connection, delivery_id: Id) -> Result<Vec<Extension>> {
    query(conn, &format!("{EXTENSION_SELECT} WHERE delivery_id = ?1 ORDER BY start_date"), &[delivery_id.into()], map_extension)
}

pub fn extension(conn: &Connection, id: Id) -> Result<Option<Extension>> {
    Ok(conn.query_row(&format!("{EXTENSION_SELECT} WHERE id = ?1"), [id], map_extension).optional()?)
}

pub fn insert_extension(
    conn: &Connection,
    delivery_id: Id,
    status: ExtensionStatus,
    start: &str,
    end: &str,
    i: &ExtensionInput,
) -> Result<Id> {
    conn.execute(
        "INSERT INTO delivery_extensions (delivery_id, status, start_date, end_date, days, daily_rate, notes, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![delivery_id, status.code(), start, end, i.days, i.daily_rate, i.notes, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_extension(conn: &Connection, id: Id, status: ExtensionStatus, start: &str, end: &str, i: &ExtensionInput) -> Result<()> {
    conn.execute(
        "UPDATE delivery_extensions SET status = ?2, start_date = ?3, end_date = ?4, days = ?5, daily_rate = ?6, notes = ?7 WHERE id = ?1",
        params![id, status.code(), start, end, i.days, i.daily_rate, i.notes],
    )?;
    Ok(())
}

pub fn delete_extension(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM delivery_extensions WHERE id = ?1", [id])?;
    Ok(())
}
