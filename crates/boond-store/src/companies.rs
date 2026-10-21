//! Sociétés.

use boond_domain::Id;
use boond_domain::model::{Company, CompanyFilter, CompanyInput, Named};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{Filter, named, query};
use crate::{Result, now};

const SELECT: &str = "SELECT c.id, c.name, c.type_id, c.sector_id, c.website, c.phone, c.email, c.address, \
     c.postal_code, c.city, c.country, c.notes, c.owner_id, u.first_name || ' ' || u.last_name, \
     c.archived_at IS NOT NULL, c.created_at, c.updated_at, \
     (SELECT COUNT(*) FROM contacts k WHERE k.company_id = c.id AND k.archived_at IS NULL), \
     (SELECT COUNT(*) FROM needs n JOIN ref_items s ON s.id = n.state_id \
        WHERE n.company_id = c.id AND n.archived_at IS NULL AND s.category = 'open') \
     FROM companies c LEFT JOIN users u ON u.id = c.owner_id";

fn map(r: &Row) -> rusqlite::Result<Company> {
    Ok(Company {
        id: r.get(0)?,
        name: r.get(1)?,
        type_id: r.get(2)?,
        sector_id: r.get(3)?,
        website: r.get(4)?,
        phone: r.get(5)?,
        email: r.get(6)?,
        address: r.get(7)?,
        postal_code: r.get(8)?,
        city: r.get(9)?,
        country: r.get(10)?,
        notes: r.get(11)?,
        owner: named(r.get(12)?, r.get(13)?),
        archived: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        contacts_count: r.get(17)?,
        open_needs_count: r.get(18)?,
    })
}

pub fn list(conn: &Connection, f: &CompanyFilter) -> Result<Vec<Company>> {
    let mut w = Filter::new();
    w.archived("c", f.archived);
    w.search("c.name || ' ' || COALESCE(c.city, '') || ' ' || COALESCE(c.website, '') || ' ' || COALESCE(c.email, '')", f.q.as_deref());
    if let Some(v) = f.type_id {
        w.add("c.type_id = ?", v);
    }
    if let Some(v) = f.sector_id {
        w.add("c.sector_id = ?", v);
    }
    if let Some(v) = f.owner_id {
        w.add("c.owner_id = ?", v);
    }
    query(conn, &format!("{SELECT}{} ORDER BY c.name COLLATE NOCASE LIMIT 1000", w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Company>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE c.id = ?1"), [id], map).optional()?)
}

pub fn insert(conn: &Connection, i: &CompanyInput, user_id: Id) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO companies (name, type_id, sector_id, website, phone, email, address, postal_code, city, \
         country, notes, owner_id, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            i.name, i.type_id, i.sector_id, i.website, i.phone, i.email, i.address, i.postal_code, i.city, i.country,
            i.notes, i.owner_id, user_id, now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, i: &CompanyInput) -> Result<()> {
    conn.execute(
        "UPDATE companies SET name = ?2, type_id = ?3, sector_id = ?4, website = ?5, phone = ?6, email = ?7, \
         address = ?8, postal_code = ?9, city = ?10, country = ?11, notes = ?12, owner_id = ?13, updated_at = ?14 \
         WHERE id = ?1",
        params![
            id, i.name, i.type_id, i.sector_id, i.website, i.phone, i.email, i.address, i.postal_code, i.city,
            i.country, i.notes, i.owner_id, now()
        ],
    )?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: Id, archived: bool) -> Result<()> {
    let now = now();
    let value = archived.then_some(now.clone());
    conn.execute("UPDATE companies SET archived_at = ?2, updated_at = ?3 WHERE id = ?1", params![id, value, now])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM companies WHERE id = ?1", [id])?;
    Ok(())
}

/// Société portant le même nom (sans tenir compte de la casse ni des accents).
pub fn find_by_name(conn: &Connection, name: &str, exclude: Option<Id>) -> Result<Option<Named>> {
    Ok(conn
        .query_row(
            "SELECT id, name FROM companies WHERE fold(trim(name)) = fold(trim(?1)) AND id IS NOT ?2 LIMIT 1",
            params![name, exclude],
            |r| Ok(Named { id: r.get(0)?, name: r.get(1)? }),
        )
        .optional()?)
}

/// Sociétés dont le nom contient la saisie, pour les rapprochements.
pub fn similar(conn: &Connection, name: &str, limit: i64) -> Result<Vec<Named>> {
    let mut w = Filter::new();
    w.raw("archived_at IS NULL");
    w.search("name", Some(name));
    w.params.push(limit.into());
    query(conn, &format!("SELECT id, name FROM companies{} ORDER BY length(name) LIMIT ?", w.sql()), &w.params, |r| {
        Ok(Named { id: r.get(0)?, name: r.get(1)? })
    })
}

/// Nombre d'objets rattachés (contacts, besoins, projets) ; une société liée ne peut pas être supprimée.
pub fn links(conn: &Connection, id: Id) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT (SELECT COUNT(*) FROM contacts WHERE company_id = ?1) + (SELECT COUNT(*) FROM needs WHERE company_id = ?1) \
         + (SELECT COUNT(*) FROM projects WHERE company_id = ?1)",
        [id],
        |r| r.get(0),
    )?)
}
