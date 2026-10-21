//! Tableau de bord, recherche globale et historique.

use xpeperience_domain::model::{ActivityFilter, Dashboard, EntityKind, HistoryEntry, SearchHit};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::{Id, Result};
use xpeperience_store::Connection;

use crate::AuthUser;
use crate::support::today_plus;

fn area_of(kind: EntityKind) -> Area {
    match kind {
        EntityKind::Company | EntityKind::Contact => Area::Crm,
        EntityKind::Need => Area::Needs,
        EntityKind::Positioning => Area::Positionings,
        EntityKind::Resource => Area::Resources,
        EntityKind::Project | EntityKind::Delivery => Area::Projects,
    }
}

fn readable(me: &AuthUser, kinds: &[EntityKind]) -> Vec<EntityKind> {
    kinds.iter().copied().filter(|k| me.can(area_of(*k), Action::Read)).collect()
}

pub fn dashboard(conn: &Connection, me: &AuthUser) -> Result<Dashboard> {
    let settings = xpeperience_store::settings::load(conn)?;
    let kinds = readable(
        me,
        &[EntityKind::Company, EntityKind::Contact, EntityKind::Need, EntityKind::Positioning, EntityKind::Project],
    );
    let reminders = if settings.reminders.enabled {
        xpeperience_store::activities::list(
            conn,
            &ActivityFilter {
                owner_id: Some(me.id),
                reminders: Some(true),
                due_before: Some(today_plus(settings.reminders.upcoming_days)),
                limit: Some(8),
                ..Default::default()
            },
        )?
    } else {
        Vec::new()
    };
    Ok(Dashboard { counts: xpeperience_store::dashboard::counts(conn)?, recent: xpeperience_store::history::recent(conn, &kinds, 12)?, reminders })
}

pub fn search(conn: &Connection, me: &AuthUser, q: &str) -> Result<Vec<SearchHit>> {
    if q.trim().chars().count() < 2 {
        return Ok(Vec::new());
    }
    let kinds = readable(
        me,
        &[EntityKind::Company, EntityKind::Contact, EntityKind::Need, EntityKind::Resource, EntityKind::Project],
    );
    Ok(xpeperience_store::search::search(conn, q, &kinds, 5)?)
}

pub fn history(conn: &Connection, me: &AuthUser, entity: EntityKind, id: Id) -> Result<Vec<HistoryEntry>> {
    me.require(area_of(entity), Action::Read)?;
    let mut entries = xpeperience_store::history::for_entity(conn, entity, id)?;
    // Les changements de coût ne sont visibles qu'avec le droit correspondant.
    if !me.can(Area::Costs, Action::Read) {
        for e in &mut entries {
            e.changes.retain(|c| !c.field.contains("CJM"));
        }
    }
    Ok(entries)
}
