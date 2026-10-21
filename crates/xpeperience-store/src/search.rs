//! Recherche globale sur les principaux objets.

use xpeperience_domain::model::{EntityKind, SearchHit};
use xpeperience_domain::text::like_pattern;
use rusqlite::Connection;

use crate::Result;
use crate::sql::query;

/// Requêtes de recherche par type d'objet : (type, SQL renvoyant id, titre, sous-titre).
fn source(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Company => {
            "SELECT id, name, city FROM companies WHERE archived_at IS NULL \
             AND fold(name || ' ' || COALESCE(city, '') || ' ' || COALESCE(website, '')) LIKE ?1 ESCAPE '\\' \
             ORDER BY name LIMIT ?2"
        }
        EntityKind::Contact => {
            "SELECT k.id, COALESCE(k.first_name || ' ', '') || k.last_name, \
             TRIM(COALESCE(k.job_title, '') || CASE WHEN co.name IS NOT NULL THEN ' · ' || co.name ELSE '' END, ' ·') \
             FROM contacts k LEFT JOIN companies co ON co.id = k.company_id WHERE k.archived_at IS NULL \
             AND fold(COALESCE(k.first_name, '') || ' ' || k.last_name || ' ' || COALESCE(k.email, '') || ' ' || COALESCE(co.name, '')) \
             LIKE ?1 ESCAPE '\\' ORDER BY k.last_name LIMIT ?2"
        }
        EntityKind::Need => {
            "SELECT n.id, n.title, co.name FROM needs n LEFT JOIN companies co ON co.id = n.company_id \
             WHERE n.archived_at IS NULL AND fold(n.title || ' ' || n.skills || ' ' || COALESCE(co.name, '')) LIKE ?1 ESCAPE '\\' \
             ORDER BY n.updated_at DESC LIMIT ?2"
        }
        EntityKind::Resource => {
            "SELECT id, first_name || ' ' || last_name, title FROM resources WHERE archived_at IS NULL \
             AND fold(first_name || ' ' || last_name || ' ' || COALESCE(title, '') || ' ' || skills) LIKE ?1 ESCAPE '\\' \
             ORDER BY last_name LIMIT ?2"
        }
        EntityKind::Project => {
            "SELECT p.id, p.title, co.name FROM projects p LEFT JOIN companies co ON co.id = p.company_id \
             WHERE p.archived_at IS NULL AND fold(p.title || ' ' || COALESCE(p.reference, '') || ' ' || COALESCE(co.name, '')) \
             LIKE ?1 ESCAPE '\\' ORDER BY p.updated_at DESC LIMIT ?2"
        }
        EntityKind::Positioning | EntityKind::Delivery => "",
    }
}

pub fn search(conn: &Connection, q: &str, kinds: &[EntityKind], per_kind: i64) -> Result<Vec<SearchHit>> {
    let pattern = like_pattern(q);
    let mut hits = Vec::new();
    for &kind in kinds {
        let sql = source(kind);
        if sql.is_empty() {
            continue;
        }
        hits.extend(query(conn, sql, &[pattern.clone().into(), per_kind.into()], |r| {
            Ok(SearchHit { kind, id: r.get(0)?, title: r.get(1)?, subtitle: r.get::<_, Option<String>>(2)?.filter(|s| !s.is_empty()) })
        })?);
    }
    Ok(hits)
}
