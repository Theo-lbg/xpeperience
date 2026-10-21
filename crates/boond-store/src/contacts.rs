//! Contacts.

use boond_domain::Id;
use boond_domain::model::{Contact, ContactFilter, ContactInput, DuplicateContact, person_name};
use boond_domain::text::{fold, linkedin_key};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::sql::{Filter, named, query};
use crate::{Result, now};

const SELECT: &str = "SELECT k.id, k.company_id, co.name, k.first_name, k.last_name, k.job_title, k.email, k.phone, \
     k.mobile, k.linkedin_url, k.location, k.notes, k.owner_id, u.first_name || ' ' || u.last_name, k.source, \
     k.imported_at, k.archived_at IS NOT NULL, k.created_at, k.updated_at \
     FROM contacts k LEFT JOIN companies co ON co.id = k.company_id LEFT JOIN users u ON u.id = k.owner_id";

fn map(r: &Row) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: r.get(0)?,
        company: named(r.get(1)?, r.get(2)?),
        first_name: r.get(3)?,
        last_name: r.get(4)?,
        job_title: r.get(5)?,
        email: r.get(6)?,
        phone: r.get(7)?,
        mobile: r.get(8)?,
        linkedin_url: r.get(9)?,
        location: r.get(10)?,
        notes: r.get(11)?,
        owner: named(r.get(12)?, r.get(13)?),
        source: r.get(14)?,
        imported_at: r.get(15)?,
        archived: r.get(16)?,
        created_at: r.get(17)?,
        updated_at: r.get(18)?,
    })
}

pub fn list(conn: &Connection, f: &ContactFilter) -> Result<Vec<Contact>> {
    let mut w = Filter::new();
    w.archived("k", f.archived);
    w.search(
        "COALESCE(k.first_name, '') || ' ' || k.last_name || ' ' || COALESCE(k.email, '') || ' ' || \
         COALESCE(k.job_title, '') || ' ' || COALESCE(co.name, '') || ' ' || COALESCE(k.location, '')",
        f.q.as_deref(),
    );
    if let Some(v) = f.company_id {
        w.add("k.company_id = ?", v);
    }
    if let Some(v) = f.owner_id {
        w.add("k.owner_id = ?", v);
    }
    if let Some(v) = f.type_id {
        w.add("co.type_id = ?", v);
    }
    if let Some(v) = f.sector_id {
        w.add("co.sector_id = ?", v);
    }
    if let Some(v) = &f.source {
        w.add("k.source = ?", v.clone());
    }
    query(conn, &format!("{SELECT}{} ORDER BY k.last_name COLLATE NOCASE, k.first_name LIMIT 1000", w.sql()), &w.params, map)
}

pub fn get(conn: &Connection, id: Id) -> Result<Option<Contact>> {
    Ok(conn.query_row(&format!("{SELECT} WHERE k.id = ?1"), [id], map).optional()?)
}

/// Provenance d'un contact créé : saisie manuelle ou import LinkedIn.
pub struct Origin<'a> {
    pub source: &'a str,
    pub imported_at: Option<&'a str>,
}

pub fn insert(conn: &Connection, i: &ContactInput, origin: Origin, user_id: Id) -> Result<Id> {
    let now = now();
    conn.execute(
        "INSERT INTO contacts (company_id, first_name, last_name, job_title, email, phone, mobile, linkedin_url, \
         location, notes, owner_id, source, imported_at, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            i.company_id, i.first_name, i.last_name, i.job_title, i.email, i.phone, i.mobile, i.linkedin_url,
            i.location, i.notes, i.owner_id, origin.source, origin.imported_at, user_id, now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: Id, i: &ContactInput) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET company_id = ?2, first_name = ?3, last_name = ?4, job_title = ?5, email = ?6, \
         phone = ?7, mobile = ?8, linkedin_url = ?9, location = ?10, notes = ?11, owner_id = ?12, updated_at = ?13 \
         WHERE id = ?1",
        params![
            id, i.company_id, i.first_name, i.last_name, i.job_title, i.email, i.phone, i.mobile, i.linkedin_url,
            i.location, i.notes, i.owner_id, now()
        ],
    )?;
    Ok(())
}

pub fn mark_imported(conn: &Connection, id: Id, at: &str) -> Result<()> {
    conn.execute("UPDATE contacts SET imported_at = ?2 WHERE id = ?1", params![id, at])?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: Id, archived: bool) -> Result<()> {
    let now = now();
    let value = archived.then_some(now.clone());
    conn.execute("UPDATE contacts SET archived_at = ?2, updated_at = ?3 WHERE id = ?1", params![id, value, now])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: Id) -> Result<()> {
    conn.execute("DELETE FROM contacts WHERE id = ?1", [id])?;
    Ok(())
}

/// Besoins et projets rattachés ; un contact lié ne peut pas être supprimé.
pub fn links(conn: &Connection, id: Id) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT (SELECT COUNT(*) FROM needs WHERE contact_id = ?1) + (SELECT COUNT(*) FROM projects WHERE contact_id = ?1)",
        [id],
        |r| r.get(0),
    )?)
}

pub struct DuplicateQuery<'a> {
    pub first_name: Option<&'a str>,
    pub last_name: &'a str,
    pub email: Option<&'a str>,
    pub linkedin_url: Option<&'a str>,
    pub company_id: Option<Id>,
    pub exclude: Option<Id>,
}

/// Contacts qui ressemblent à la saisie : même profil LinkedIn, même e-mail ou même nom.
pub fn duplicates(conn: &Connection, q: &DuplicateQuery) -> Result<Vec<DuplicateContact>> {
    let key = q.linkedin_url.and_then(linkedin_key);
    let slug = key.as_deref().map(|k| format!("%{}%", k.trim_start_matches("linkedin.com/in/")));
    let rows = query(
        conn,
        "SELECT k.id, k.first_name, k.last_name, co.name, k.job_title, k.email, k.linkedin_url, k.company_id \
         FROM contacts k LEFT JOIN companies co ON co.id = k.company_id \
         WHERE k.id IS NOT ?1 AND ( \
           (?2 IS NOT NULL AND k.email = ?2 COLLATE NOCASE) \
           OR (?3 IS NOT NULL AND lower(k.linkedin_url) LIKE ?3) \
           OR fold(trim(k.last_name)) = fold(trim(?4))) \
         LIMIT 50",
        &[q.exclude.into(), q.email.map(str::to_string).into(), slug.into(), q.last_name.to_string().into()],
        |r| {
            Ok((
                r.get::<_, Id>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<Id>>(7)?,
            ))
        },
    )?;

    let wanted_first = q.first_name.map(fold);
    let mut out = Vec::new();
    for (id, first, last, company, job, email, linkedin, company_id) in rows {
        let same_linkedin = key.is_some() && linkedin.as_deref().and_then(linkedin_key) == key;
        let same_email = q.email.is_some_and(|e| email.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(e)));
        let same_first = match (&wanted_first, first.as_deref().map(fold)) {
            (Some(a), Some(b)) => *a == b,
            _ => true,
        };
        let reason = if same_linkedin {
            "Même profil LinkedIn"
        } else if same_email {
            "Même adresse e-mail"
        } else if same_first && q.company_id.is_some() && company_id == q.company_id {
            "Même nom dans la même société"
        } else if same_first {
            "Même nom"
        } else {
            continue;
        };
        out.push(DuplicateContact {
            id,
            name: person_name(first.as_deref(), &last),
            company,
            job_title: job,
            reason: reason.into(),
        });
    }
    Ok(out)
}
