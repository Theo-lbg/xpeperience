//! Chiffres clés du tableau de bord.

use boond_domain::model::DashboardCounts;
use rusqlite::Connection;

use crate::Result;

pub fn counts(conn: &Connection) -> Result<DashboardCounts> {
    Ok(conn.query_row(
        "SELECT \
           (SELECT COUNT(*) FROM companies WHERE archived_at IS NULL), \
           (SELECT COUNT(*) FROM contacts WHERE archived_at IS NULL), \
           (SELECT COUNT(*) FROM needs n JOIN ref_items s ON s.id = n.state_id WHERE n.archived_at IS NULL AND s.category = 'open'), \
           (SELECT COUNT(*) FROM resources WHERE archived_at IS NULL AND kind = 'candidate'), \
           (SELECT COUNT(*) FROM resources WHERE archived_at IS NULL AND kind = 'consultant')",
        [],
        |r| {
            let candidates: i64 = r.get(3)?;
            let consultants: i64 = r.get(4)?;
            Ok(DashboardCounts {
                companies: r.get(0)?,
                contacts: r.get(1)?,
                open_needs: r.get(2)?,
                resources: candidates + consultants,
                candidates,
                consultants,
            })
        },
    )?)
}
