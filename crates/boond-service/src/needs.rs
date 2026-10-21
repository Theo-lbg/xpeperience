//! Besoins clients : création, suivi des états et passage en projet.

use boond_domain::error::Errors;
use boond_domain::model::{EntityKind, Need, NeedFilter, NeedInput, Project, ProjectInput};
use boond_domain::permissions::{Action, Area};
use boond_domain::refs::{RefList, project_kind_for_need};
use boond_domain::rules::{check_amount, check_date, check_required};
use boond_domain::text::{clean, clean_required, clean_tags};
use boond_domain::{DomainError, Id, Result};
use boond_store::Connection;

use crate::AuthUser;
use crate::support::{Diff, Refs, Trace, begin, check_user, commit, trace};

pub fn needs(conn: &Connection, me: &AuthUser, f: &NeedFilter) -> Result<Vec<Need>> {
    me.require(Area::Needs, Action::Read)?;
    Ok(boond_store::needs::list(conn, f)?)
}

pub fn need(conn: &Connection, me: &AuthUser, id: Id) -> Result<Need> {
    me.require(Area::Needs, Action::Read)?;
    boond_store::needs::get(conn, id)?.ok_or(DomainError::NotFound("Besoin"))
}

/// Nettoie et valide un besoin. Renvoie aussi la société déduite du contact.
fn clean_need(conn: &Connection, refs: &Refs, i: NeedInput) -> Result<(NeedInput, Option<Id>)> {
    let settings = boond_store::settings::load(conn)?;
    let i = NeedInput {
        title: clean_required(&i.title),
        contact_id: i.contact_id,
        description: clean(i.description),
        skills: clean_tags(i.skills),
        type_id: i.type_id,
        state_id: i.state_id,
        origin_id: i.origin_id,
        budget: i.budget,
        duration_months: i.duration_months,
        planned_days: i.planned_days,
        start_date: clean(i.start_date),
        location: clean(i.location),
        owner_id: i.owner_id,
    };
    let mut errors = Errors::new();
    if i.title.is_empty() {
        errors.add("title", "Donnez un titre au besoin.");
    }
    let mut company_id = None;
    match i.contact_id {
        None => errors.add("contact_id", "Rattachez le besoin à un contact du CRM."),
        Some(cid) => match boond_store::contacts::get(conn, cid)? {
            Some(c) => company_id = c.company.map(|co| co.id),
            None => errors.add("contact_id", "Contact introuvable."),
        },
    }
    refs.check(&mut errors, "type_id", RefList::NeedType, i.type_id);
    refs.check(&mut errors, "state_id", RefList::NeedState, i.state_id);
    refs.check(&mut errors, "origin_id", RefList::NeedOrigin, i.origin_id);
    check_amount(&mut errors, "budget", i.budget);
    check_amount(&mut errors, "duration_months", i.duration_months);
    check_amount(&mut errors, "planned_days", i.planned_days);
    check_date(&mut errors, "start_date", i.start_date.as_deref());
    check_user(conn, &mut errors, "owner_id", i.owner_id)?;
    check_required(
        &mut errors,
        &settings,
        "need",
        &[
            ("description", i.description.is_some()),
            ("skills", !i.skills.is_empty()),
            ("type_id", i.type_id.is_some()),
            ("origin_id", i.origin_id.is_some()),
            ("budget", i.budget.is_some()),
            ("duration_months", i.duration_months.is_some()),
            ("planned_days", i.planned_days.is_some()),
            ("start_date", i.start_date.is_some()),
            ("owner_id", i.owner_id.is_some()),
        ],
    );
    errors.finish()?;
    Ok((i, company_id))
}

fn default_state(refs: &Refs) -> Result<Id> {
    refs.first(RefList::NeedState, Some("open"))
        .ok_or_else(|| DomainError::rule("Aucun état « En cours » actif pour les besoins : vérifiez les réglages."))
}

pub fn create_need(conn: &mut Connection, me: &AuthUser, input: NeedInput) -> Result<Need> {
    me.require(Area::Needs, Action::Create)?;
    let tx = begin(conn)?;
    let refs = Refs::load(&tx)?;
    let (mut input, company_id) = clean_need(&tx, &refs, input)?;
    let state_id = match input.state_id {
        Some(s) => s,
        None => default_state(&refs)?,
    };
    input.owner_id = input.owner_id.or(Some(me.id));
    let id = boond_store::needs::insert(&tx, &input, company_id, state_id, me.id)?;
    let link = company_id.map(|c| (EntityKind::Company, c));
    trace(&tx, me, Trace { entity: EntityKind::Need, id, label: &input.title, link }, "created", &[])?;
    commit(tx)?;
    boond_store::needs::get(conn, id)?.ok_or(DomainError::NotFound("Besoin"))
}

fn need_diff(refs: &Refs, old: &Need, new: &Need) -> Diff {
    let mut d = Diff::new();
    d.text("Titre", Some(&old.title), Some(&new.title));
    d.named("Contact", old.contact.as_ref(), new.contact.as_ref());
    d.named("Société", old.company.as_ref(), new.company.as_ref());
    d.reference("État", refs, Some(old.state_id), Some(new.state_id));
    d.reference("Type", refs, old.type_id, new.type_id);
    d.reference("Provenance", refs, old.origin_id, new.origin_id);
    d.text("Description", old.description.as_deref(), new.description.as_deref());
    d.tags("Compétences", &old.skills, &new.skills);
    d.num("Budget", old.budget, new.budget);
    d.num("Durée (mois)", old.duration_months, new.duration_months);
    d.num("Jours prévus", old.planned_days, new.planned_days);
    d.text("Démarrage souhaité", old.start_date.as_deref(), new.start_date.as_deref());
    d.text("Lieu", old.location.as_deref(), new.location.as_deref());
    d.named("Responsable commercial", old.owner.as_ref(), new.owner.as_ref());
    d
}

fn trace_need_change(conn: &Connection, me: &AuthUser, refs: &Refs, old: &Need, new: &Need) -> Result<()> {
    let d = need_diff(refs, old, new);
    if d.is_empty() {
        return Ok(());
    }
    let action = if d.only_field("État") { "state_changed" } else { "updated" };
    let link = new.company.as_ref().map(|c| (EntityKind::Company, c.id));
    trace(conn, me, Trace { entity: EntityKind::Need, id: new.id, label: &new.title, link }, action, &d.into_vec())
}

pub fn update_need(conn: &mut Connection, me: &AuthUser, id: Id, input: NeedInput) -> Result<Need> {
    me.require(Area::Needs, Action::Update)?;
    let tx = begin(conn)?;
    let old = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    let refs = Refs::load(&tx)?;
    let (input, company_id) = clean_need(&tx, &refs, input)?;
    boond_store::needs::update(&tx, id, &input, company_id, input.state_id.unwrap_or(old.state_id))?;
    let new = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    trace_need_change(&tx, me, &refs, &old, &new)?;
    commit(tx)?;
    Ok(new)
}

pub fn set_state(conn: &mut Connection, me: &AuthUser, id: Id, state_id: Id) -> Result<Need> {
    me.require(Area::Needs, Action::Update)?;
    let tx = begin(conn)?;
    let old = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    let refs = Refs::load(&tx)?;
    let mut errors = Errors::new();
    refs.check(&mut errors, "state_id", RefList::NeedState, Some(state_id));
    errors.finish()?;
    boond_store::needs::set_state(&tx, id, state_id)?;
    let new = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    trace_need_change(&tx, me, &refs, &old, &new)?;
    commit(tx)?;
    Ok(new)
}

pub fn archive_need(conn: &mut Connection, me: &AuthUser, id: Id, archived: bool) -> Result<Need> {
    me.require(Area::Needs, Action::Update)?;
    let tx = begin(conn)?;
    let n = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    boond_store::needs::set_archived(&tx, id, archived)?;
    let link = n.company.as_ref().map(|c| (EntityKind::Company, c.id));
    let action = if archived { "archived" } else { "restored" };
    trace(&tx, me, Trace { entity: EntityKind::Need, id, label: &n.title, link }, action, &[])?;
    commit(tx)?;
    boond_store::needs::get(conn, id)?.ok_or(DomainError::NotFound("Besoin"))
}

pub fn delete_need(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Needs, Action::Delete)?;
    let tx = begin(conn)?;
    let n = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    if let Some(p) = &n.project {
        return Err(DomainError::rule(format!(
            "Ce besoin a donné lieu au projet « {} » : archivez-le plutôt que de le supprimer.",
            p.name
        )));
    }
    boond_store::needs::delete(&tx, id)?;
    trace(&tx, me, Trace { entity: EntityKind::Need, id, label: &n.title, link: None }, "deleted", &[])?;
    commit(tx)
}

/// Crée un projet à partir d'un besoin en reprenant ses informations.
pub(crate) fn project_from_need(conn: &Connection, me: &AuthUser, refs: &Refs, need: &Need) -> Result<Id> {
    let kind = project_kind_for_need(refs.category(need.type_id));
    let input = ProjectInput {
        reference: None,
        title: need.title.clone(),
        company_id: need.company.as_ref().map(|c| c.id),
        contact_id: need.contact.as_ref().map(|c| c.id),
        type_id: refs.first(RefList::ProjectType, Some(kind)),
        state_id: refs.first(RefList::ProjectState, Some("planned")).or_else(|| refs.first(RefList::ProjectState, None)),
        manager_id: need.owner.as_ref().map(|o| o.id),
        start_date: need.start_date.clone(),
        end_date: None,
        notes: need.description.clone(),
    };
    let id = boond_store::projects::insert(conn, &input, Some(need.id), me.id)?;
    boond_store::needs::set_project(conn, need.id, id)?;
    trace(
        conn,
        me,
        Trace { entity: EntityKind::Project, id, label: &input.title, link: Some((EntityKind::Need, need.id)) },
        "created",
        &[],
    )?;
    Ok(id)
}

/// Passe le besoin dans le premier état « gagné », si les réglages le prévoient.
pub(crate) fn mark_won(conn: &Connection, me: &AuthUser, refs: &Refs, need_id: Id) -> Result<()> {
    let settings = boond_store::settings::load(conn)?;
    if !settings.business.mark_need_won {
        return Ok(());
    }
    let old = boond_store::needs::get(conn, need_id)?.ok_or(DomainError::NotFound("Besoin"))?;
    if refs.category(Some(old.state_id)) == Some("won") {
        return Ok(());
    }
    if let Some(won) = refs.first(RefList::NeedState, Some("won")) {
        boond_store::needs::set_state(conn, need_id, won)?;
        let new = boond_store::needs::get(conn, need_id)?.ok_or(DomainError::NotFound("Besoin"))?;
        trace_need_change(conn, me, refs, &old, &new)?;
    }
    Ok(())
}

/// Transforme un besoin (forfait, produit…) en projet, sans passer par un positionnement.
pub fn convert_need(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<Project> {
    me.require(Area::Projects, Action::Create)?;
    me.require(Area::Needs, Action::Update)?;
    let tx = begin(conn)?;
    let need = boond_store::needs::get(&tx, id)?.ok_or(DomainError::NotFound("Besoin"))?;
    if let Some(p) = &need.project {
        return Err(DomainError::conflict(
            format!("Ce besoin est déjà rattaché au projet « {} ».", p.name),
            Some(serde_json::json!({ "entity": "project", "id": p.id })),
        ));
    }
    let refs = Refs::load(&tx)?;
    let project_id = project_from_need(&tx, me, &refs, &need)?;
    mark_won(&tx, me, &refs, id)?;
    commit(tx)?;
    boond_store::projects::get(conn, project_id)?.ok_or(DomainError::NotFound("Projet"))
}
