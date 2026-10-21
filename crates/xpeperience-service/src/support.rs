//! Outils partagés : listes paramétrables, différences pour l'historique, masquage des coûts.

use std::collections::HashMap;

use xpeperience_domain::error::Errors;
use xpeperience_domain::model::{Delivery, EntityKind, FieldChange, HrContract, Named, Positioning, Resource};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::refs::{RefItem, RefList};
use xpeperience_domain::{Id, Result};
use xpeperience_store::Connection;
use xpeperience_store::history::{NewEntry, record};

use crate::AuthUser;

/// Listes paramétrables chargées en mémoire le temps d'une opération.
pub(crate) struct Refs(HashMap<Id, RefItem>);

impl Refs {
    pub fn load(conn: &Connection) -> Result<Self> {
        Ok(Self(xpeperience_store::refs::all(conn)?.into_iter().map(|r| (r.id, r)).collect()))
    }

    pub fn label(&self, id: Option<Id>) -> Option<String> {
        id.and_then(|id| self.0.get(&id)).map(|r| r.label.clone())
    }

    pub fn category(&self, id: Option<Id>) -> Option<&str> {
        id.and_then(|id| self.0.get(&id)).and_then(|r| r.category.as_deref())
    }

    /// Vérifie qu'un identifiant appartient bien à la liste attendue.
    pub fn check(&self, errors: &mut Errors, field: &str, list: RefList, id: Option<Id>) {
        if let Some(id) = id
            && self.0.get(&id).is_none_or(|r| r.list != list)
        {
            errors.add(field, "Valeur inconnue : rechargez la page.");
        }
    }

    /// Premier élément actif d'une liste (éventuellement d'une catégorie), selon l'ordre défini.
    pub fn first(&self, list: RefList, category: Option<&str>) -> Option<Id> {
        self.0
            .values()
            .filter(|r| r.list == list && r.active && category.is_none_or(|c| r.category.as_deref() == Some(c)))
            .min_by_key(|r| (r.position, r.id))
            .map(|r| r.id)
    }
}

/// Différences entre l'ancien et le nouvel état d'un objet, pour l'historique.
#[derive(Default)]
pub(crate) struct Diff(Vec<FieldChange>);

impl Diff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&mut self, field: &str, old: Option<&str>, new: Option<&str>) {
        if old != new {
            self.0.push(FieldChange { field: field.into(), from: old.map(Into::into), to: new.map(Into::into) });
        }
    }

    pub fn num(&mut self, field: &str, old: Option<f64>, new: Option<f64>) {
        if old != new {
            self.0.push(FieldChange { field: field.into(), from: old.map(|v| v.to_string()), to: new.map(|v| v.to_string()) });
        }
    }

    pub fn reference(&mut self, field: &str, refs: &Refs, old: Option<Id>, new: Option<Id>) {
        if old != new {
            self.0.push(FieldChange { field: field.into(), from: refs.label(old), to: refs.label(new) });
        }
    }

    pub fn named(&mut self, field: &str, old: Option<&Named>, new: Option<&Named>) {
        if old.map(|n| n.id) != new.map(|n| n.id) {
            self.0.push(FieldChange {
                field: field.into(),
                from: old.map(|n| n.name.clone()),
                to: new.map(|n| n.name.clone()),
            });
        }
    }

    pub fn tags(&mut self, field: &str, old: &[String], new: &[String]) {
        if old != new {
            let join = |t: &[String]| if t.is_empty() { None } else { Some(t.join(", ")) };
            self.0.push(FieldChange { field: field.into(), from: join(old), to: join(new) });
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn only_field(&self, field: &str) -> bool {
        self.0.len() == 1 && self.0[0].field == field
    }

    pub fn into_vec(self) -> Vec<FieldChange> {
        self.0
    }
}

pub(crate) struct Trace<'a> {
    pub entity: EntityKind,
    pub id: Id,
    pub label: &'a str,
    pub link: Option<(EntityKind, Id)>,
}

pub(crate) fn trace(conn: &Connection, me: &AuthUser, t: Trace, action: &str, changes: &[FieldChange]) -> Result<()> {
    record(
        conn,
        &NewEntry {
            entity: t.entity,
            entity_id: t.id,
            entity_label: t.label,
            action,
            changes,
            link: t.link,
            user_id: me.id,
        },
    )?;
    Ok(())
}

/// Nom d'un utilisateur existant et actif, pour valider un responsable.
pub(crate) fn check_user(conn: &Connection, errors: &mut Errors, field: &str, id: Option<Id>) -> Result<Option<Named>> {
    let Some(id) = id else { return Ok(None) };
    match xpeperience_store::users::get(conn, id)? {
        Some(u) => Ok(Some(Named { id: u.id, name: format!("{} {}", u.first_name, u.last_name) })),
        None => {
            errors.add(field, "Utilisateur inconnu.");
            Ok(None)
        }
    }
}

// ─────────────── Masquage des coûts (CJM) pour les rôles sans accès ───────────────

pub(crate) fn can_see_costs(me: &AuthUser) -> bool {
    me.can(Area::Costs, Action::Read)
}

pub(crate) fn can_edit_costs(me: &AuthUser) -> bool {
    me.can(Area::Costs, Action::Update)
}

pub(crate) fn hide_resource_costs(me: &AuthUser, r: &mut Resource) {
    if !can_see_costs(me) {
        r.daily_cost = None;
        r.effective_daily_cost = None;
        r.daily_cost_source = None;
    }
}

pub(crate) fn hide_positioning_costs(me: &AuthUser, p: &mut Positioning) {
    if !can_see_costs(me) {
        p.daily_cost = None;
    }
}

pub(crate) fn hide_delivery_costs(me: &AuthUser, d: &mut Delivery) {
    if !can_see_costs(me) {
        d.daily_cost = None;
    }
}

pub(crate) fn hide_contract_costs(me: &AuthUser, c: &mut HrContract) {
    if !can_see_costs(me) {
        c.daily_cost = None;
    }
}

pub(crate) fn begin(conn: &mut Connection) -> Result<xpeperience_store::Transaction<'_>> {
    Ok(conn.transaction().map_err(xpeperience_store::StoreError::from)?)
}

pub(crate) fn commit(tx: xpeperience_store::Transaction<'_>) -> Result<()> {
    Ok(tx.commit().map_err(xpeperience_store::StoreError::from)?)
}

/// Date du jour + `days` jours, au format `AAAA-MM-JJ`.
pub(crate) fn today_plus(days: i64) -> String {
    (chrono::Local::now().date_naive() + chrono::Duration::days(days)).format("%Y-%m-%d").to_string()
}
