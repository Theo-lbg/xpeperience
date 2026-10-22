//! Ressources (candidats et consultants) et documents.

use xpeperience_domain::Id;
use xpeperience_domain::model::{Resource, ResourceFile, ResourceFilter, ResourceInput, ResourceKind};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{DELIVERY_EFFECTIVE_END, Filter, iso_literal, named, query, tags_from_json, tags_to_json};
use crate::{Result, now, today};

/// Requête de base. `mission_until` vaut la fin de la prestation en cours (ou `9999-12-31`
/// pour une prestation sans fin).
fn select(today: &str) -> String {
    let t = iso_literal(today);
    format!(
        "WITH base AS (SELECT r.*, \
           (SELECT MAX({DELIVERY_EFFECTIVE_END}) FROM deliveries d \
              WHERE d.resource_id = r.id AND d.start_date <= '{t}' AND ({DELIVERY_EFFECTIVE_END}) >= '{t}') AS mission_until, \
           (SELECT COUNT(*) FROM positionings x JOIN ref_items s ON s.id = x.state_id \
              WHERE x.resource_id = r.id AND s.category = 'open') AS open_positionings \
         FROM resources r) \
         SELECT b.id, b.kind, b.first_name, b.last_name, b.title, b.email, b.phone, b.location, b.linkedin_url, b.skills, \
           b.experience_years, b.experience, b.availability_date, b.availability_note, b.contract_type_id, b.daily_cost, \
           b.notes, b.owner_id, u.first_name || ' ' || u.last_name, b.archived_at IS NOT NULL, \
           b.created_at, b.updated_at, b.mission_until, b.open_positionings \
         FROM base b LEFT JOIN users u ON u.id = b.owner_id"
    )
}

fn map(r: &Row) -> rusqlite::Result<Resource> {
    let kind: String = r.get(1)?;
    let mission: Option<String> = r.get(22)?;
    Ok(Resource {
        id: r.get(0)?,
        kind: ResourceKind::from_code(&kind).unwrap_or(ResourceKind::Candidate),
        first_name: r.get(2)?,
        last_name: r.get(3)?,
        title: r.get(4)?,
        email: r.get(5)?,
        phone: r.get(6)?,
        location: r.get(7)?,
        linkedin_url: r.get(8)?,
        skills: tags_from_json(r.get(9)?),
        experience_years: r.get(10)?,
        experience: r.get(11)?,
        availability_date: r.get(12)?,
        availability_note: r.get(13)?,
        contract_type_id: r.get(14)?,
        daily_cost: r.get(15)?,
        notes: r.get(16)?,
        owner: named(r.get(17)?, r.get(18)?),
        archived: r.get(19)?,
        created_at: r.get(20)?,
        updated_at: r.get(21)?,
        on_mission: mission.is_some(),
        on_mission_until: mission.filter(|m| m != "9999-12-31"),
        open_positionings: r.get(23)?,
    })
}

pub fn list(conn: &Connection, f: &ResourceFilter) -> Result<Vec<Resource>> {
    let today = today();
    let mut w = Filter::new();
    w.archived("b", f.archived);
    w.search(
        "b.first_name || ' ' || b.last_name || ' ' || COALESCE(b.title, '') || ' ' || b.skills || ' ' || \
         COALESCE(b.location, '') || ' ' || COALESCE(b.email, '')",
        f.q.as_deref(),
    );
    w.search("b.skills", f.skill.as_deref());
    w.search("COALESCE(b.location, '')", f.location.as_deref());
    if let Some(k) = f.kind {
        w.add("b.kind = ?", k.code().to_string());
    }
    if let Some(v) = f.contract_type_id {
        w.add("b.contract_type_id = ?", v);
    }
    let in_30_days = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
        .map(|d| (d + chrono::Duration::days(30)).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| today.clone());
    match f.availability.as_deref() {
        Some("now") => w.add("b.mission_until IS NULL AND (b.availability_date IS NULL OR b.availability_date <= ?)", today.clone()),
        Some("soon") => w.add_many(
            "((b.mission_until IS NULL AND (b.availability_date IS NULL OR b.availability_date <= ?)) OR b.mission_until <= ?)",
            vec![in_30_days.clone().into(), in_30_days.into()],
        ),
        Some("mission") => w.raw("b.mission_until IS NOT NULL"),
        _ => {}
    }
    query(conn, &format!("{}{} ORDER BY b.last_name COLLATE NOCASE, b.first_name LIMIT 1000", select(&today), w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Resource>> {
    Ok(conn.query_row(&format!("{} WHERE b.id = ?1", select(&today())), [id], map).optional()?)
}

pub fn insert(conn: &Connection, i: &ResourceInput, kind: ResourceKind, user_id: Id) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO resources (kind, first_name, last_name, title, email, phone, location, linkedin_url, skills, \
         experience_years, experience, availability_date, availability_note, contract_type_id, daily_cost, notes, \
         owner_id, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?19)",
        params![
            kind.code(), i.first_name, i.last_name, i.title, i.email, i.phone, i.location, i.linkedin_url,
            tags_to_json(&i.skills), i.experience_years, i.experience, i.availability_date, i.availability_note,
            i.contract_type_id, i.daily_cost, i.notes, i.owner_id, user_id, now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, i: &ResourceInput, kind: ResourceKind) -> Result<()> {
    conn.execute(
        "UPDATE resources SET kind = ?2, first_name = ?3, last_name = ?4, title = ?5, email = ?6, phone = ?7, \
         location = ?8, linkedin_url = ?9, skills = ?10, experience_years = ?11, experience = ?12, \
         availability_date = ?13, availability_note = ?14, contract_type_id = ?15, daily_cost = ?16, notes = ?17, \
         owner_id = ?18, updated_at = ?19 WHERE id = ?1",
        params![
            id, kind.code(), i.first_name, i.last_name, i.title, i.email, i.phone, i.location, i.linkedin_url,
            tags_to_json(&i.skills), i.experience_years, i.experience, i.availability_date, i.availability_note,
            i.contract_type_id, i.daily_cost, i.notes, i.owner_id, now()
        ],
    )?;
    Ok(())
}

pub fn set_kind(conn: &Connection, id: Id, kind: ResourceKind) -> Result<()> {
    conn.execute("UPDATE resources SET kind = ?2, updated_at = ?3 WHERE id = ?1", params![id, kind.code(), now()])?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: Id, archived: bool) -> Result<()> {
    let now = now();
    let value = archived.then_some(now.clone());
    conn.execute("UPDATE resources SET archived_at = ?2, updated_at = ?3 WHERE id = ?1", params![id, value, now])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM resources WHERE id = ?1", [id])?;
    Ok(())
}

/// Positionnements et prestations rattachés ; une ressource liée ne peut pas être supprimée.
pub fn links(conn: &Connection, id: Id) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT (SELECT COUNT(*) FROM positionings WHERE resource_id = ?1) + (SELECT COUNT(*) FROM deliveries WHERE resource_id = ?1)",
        [id],
        |r| r.get(0),
    )?)
}

/// Ressource portant le même e-mail, pour éviter les doublons.
pub fn find_by_email(conn: &Connection, email: &str, exclude: Option<Id>) -> Result<Option<(Id, String)>> {
    Ok(conn
        .query_row(
            "SELECT id, first_name || ' ' || last_name FROM resources WHERE email = ?1 COLLATE NOCASE AND id IS NOT ?2 LIMIT 1",
            params![email, exclude],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

// ───────────────────────────── Documents ─────────────────────────────

const FILE_SELECT: &str = "SELECT id, resource_id, kind, filename, mime, size, uploaded_at, stored_name FROM resource_files";

fn map_file(r: &Row) -> rusqlite::Result<ResourceFile> {
    Ok(ResourceFile {
        id: r.get(0)?,
        resource_id: r.get(1)?,
        kind: r.get(2)?,
        filename: r.get(3)?,
        mime: r.get(4)?,
        size: r.get(5)?,
        uploaded_at: r.get(6)?,
        stored_name: r.get(7)?,
    })
}

pub fn files(conn: &Connection, resource_id: Id) -> Result<Vec<ResourceFile>> {
    query(conn, &format!("{FILE_SELECT} WHERE resource_id = ?1 ORDER BY uploaded_at DESC"), &[resource_id.into()], map_file)
}

pub fn file(conn: &Connection, id: Id) -> Result<Option<ResourceFile>> {
    Ok(conn.query_row(&format!("{FILE_SELECT} WHERE id = ?1"), [id], map_file).optional()?)
}

pub struct NewFile<'a> {
    pub resource_id: Id,
    pub kind: &'a str,
    pub filename: &'a str,
    pub stored_name: &'a str,
    pub mime: &'a str,
    pub size: i64,
}

pub fn insert_file(conn: &Connection, f: &NewFile, user_id: Id) -> Result<Id> {
    conn.execute(
        "INSERT INTO resource_files (resource_id, kind, filename, stored_name, mime, size, uploaded_by, uploaded_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![f.resource_id, f.kind, f.filename, f.stored_name, f.mime, f.size, user_id, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_file(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM resource_files WHERE id = ?1", [id])?;
    Ok(())
}
