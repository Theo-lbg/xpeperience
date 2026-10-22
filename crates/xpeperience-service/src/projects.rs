//! Projets, prestations et prolongations.

use xpeperience_domain::error::Errors;
use xpeperience_domain::model::{
    Delivery, DeliveryFilter, DeliveryInput, EntityKind, ExtensionInput, ExtensionStatus, Named, Project, ProjectFilter,
    ProjectInput,
};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::refs::RefList;
use xpeperience_domain::rules::{DeliverySpan, check_amount, check_date, check_extension, check_range, check_required, project_dates};
use xpeperience_domain::text::{clean, clean_required};
use xpeperience_domain::{DomainError, Id, Result};
use xpeperience_store::Connection;

use crate::AuthUser;
use crate::support::{Diff, Refs, Trace, begin, can_edit_costs, check_user, commit, hide_delivery_costs, trace};

pub fn projects(conn: &Connection, me: &AuthUser, f: &ProjectFilter) -> Result<Vec<Project>> {
    me.require(Area::Projects, Action::Read)?;
    Ok(xpeperience_store::projects::list(conn, f)?)
}

pub fn project(conn: &Connection, me: &AuthUser, id: Id) -> Result<Project> {
    me.require(Area::Projects, Action::Read)?;
    xpeperience_store::projects::get(conn, id)?.ok_or(DomainError::NotFound("Projet"))
}

/// Projets en cours d'une société, pour y rattacher un positionnement gagné.
pub fn open_projects(conn: &Connection, me: &AuthUser, company_id: Id) -> Result<Vec<Named>> {
    me.require(Area::Projects, Action::Read)?;
    Ok(xpeperience_store::projects::open_for_company(conn, company_id)?)
}

/// Recalcule les dates du projet à partir de ses prestations (si elles existent).
pub(crate) fn recompute_dates(conn: &Connection, project_id: Id) -> Result<()> {
    let deliveries = xpeperience_store::projects::deliveries(conn, &DeliveryFilter { project_id: Some(project_id), resource_id: None })?;
    if deliveries.is_empty() {
        return Ok(());
    }
    let spans: Vec<DeliverySpan> = deliveries
        .iter()
        .map(|d| DeliverySpan {
            start: &d.start_date,
            effective_end: d.effective_end_date.as_deref(),
            forecast_end: d.forecast_end_date.as_deref(),
        })
        .collect();
    let (start, end, forecast) = project_dates(&spans);
    xpeperience_store::projects::set_dates(conn, project_id, start.as_deref(), end.as_deref(), forecast.as_deref())?;
    xpeperience_store::projects::touch(conn, project_id)?;
    Ok(())
}

fn clean_project(conn: &Connection, refs: &Refs, i: ProjectInput) -> Result<ProjectInput> {
    let settings = xpeperience_store::settings::load(conn)?;
    let i = ProjectInput {
        reference: clean(i.reference),
        title: clean_required(&i.title),
        company_id: i.company_id,
        contact_id: i.contact_id,
        type_id: i.type_id,
        state_id: i.state_id,
        manager_id: i.manager_id,
        start_date: clean(i.start_date),
        end_date: clean(i.end_date),
        notes: clean(i.notes),
    };
    let mut errors = Errors::new();
    if i.title.is_empty() {
        errors.add("title", "Donnez un titre au projet.");
    }
    refs.check(&mut errors, "type_id", RefList::ProjectType, i.type_id);
    refs.check(&mut errors, "state_id", RefList::ProjectState, i.state_id);
    let internal = refs.category(i.type_id) == Some("interne");
    match i.company_id {
        None if !internal => errors.add("company_id", "Choisissez la société cliente (sauf pour un projet interne)."),
        Some(cid) if xpeperience_store::companies::get(conn, cid)?.is_none() => errors.add("company_id", "Société introuvable."),
        _ => {}
    }
    if let Some(cid) = i.contact_id {
        match xpeperience_store::contacts::get(conn, cid)? {
            None => errors.add("contact_id", "Contact introuvable."),
            Some(c) if i.company_id.is_some() && c.company.as_ref().map(|co| co.id) != i.company_id => {
                errors.add("contact_id", "Ce contact n'appartient pas à la société choisie.")
            }
            _ => {}
        }
    }
    check_user(conn, &mut errors, "manager_id", i.manager_id)?;
    check_date(&mut errors, "start_date", i.start_date.as_deref());
    check_date(&mut errors, "end_date", i.end_date.as_deref());
    check_range(&mut errors, "end_date", i.start_date.as_deref(), i.end_date.as_deref());
    check_required(
        &mut errors,
        &settings,
        "project",
        &[
            ("reference", i.reference.is_some()),
            ("contact_id", i.contact_id.is_some()),
            ("type_id", i.type_id.is_some()),
            ("manager_id", i.manager_id.is_some()),
        ],
    );
    errors.finish()?;
    Ok(i)
}

fn trace_project(conn: &Connection, me: &AuthUser, p: &Project, action: &str, changes: Vec<xpeperience_domain::model::FieldChange>) -> Result<()> {
    let link = p.company.as_ref().map(|c| (EntityKind::Company, c.id));
    trace(conn, me, Trace { entity: EntityKind::Project, id: p.id, label: &p.title, link }, action, &changes)
}

pub fn create_project(conn: &mut Connection, me: &AuthUser, input: ProjectInput) -> Result<Project> {
    me.require(Area::Projects, Action::Create)?;
    let tx = begin(conn)?;
    let refs = Refs::load(&tx)?;
    let mut input = clean_project(&tx, &refs, input)?;
    input.state_id = input
        .state_id
        .or_else(|| refs.first(RefList::ProjectState, Some("planned")))
        .or_else(|| refs.first(RefList::ProjectState, None));
    input.manager_id = input.manager_id.or(Some(me.id));
    let id = xpeperience_store::projects::insert(&tx, &input, None, me.id)?;
    let p = xpeperience_store::projects::get(&tx, id)?.ok_or(DomainError::NotFound("Projet"))?;
    trace_project(&tx, me, &p, "created", Vec::new())?;
    commit(tx)?;
    Ok(p)
}

pub fn update_project(conn: &mut Connection, me: &AuthUser, id: Id, input: ProjectInput) -> Result<Project> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let old = xpeperience_store::projects::get(&tx, id)?.ok_or(DomainError::NotFound("Projet"))?;
    let refs = Refs::load(&tx)?;
    let input = clean_project(&tx, &refs, input)?;
    xpeperience_store::projects::update(&tx, id, &input)?;
    if old.deliveries_count == 0 {
        // Sans prestation, les dates restent saisies à la main.
        xpeperience_store::projects::set_dates(&tx, id, input.start_date.as_deref(), input.end_date.as_deref(), input.end_date.as_deref())?;
    }
    let new = xpeperience_store::projects::get(&tx, id)?.ok_or(DomainError::NotFound("Projet"))?;

    let mut d = Diff::new();
    d.text("Titre", Some(&old.title), Some(&new.title));
    d.text("Référence", old.reference.as_deref(), new.reference.as_deref());
    d.named("Société", old.company.as_ref(), new.company.as_ref());
    d.named("Contact", old.contact.as_ref(), new.contact.as_ref());
    d.reference("Type", &refs, old.type_id, new.type_id);
    d.reference("État", &refs, old.state_id, new.state_id);
    d.named("Responsable", old.manager.as_ref(), new.manager.as_ref());
    d.text("Début", old.start_date.as_deref(), new.start_date.as_deref());
    d.text("Fin", old.end_date.as_deref(), new.end_date.as_deref());
    d.text("Notes", old.notes.as_deref(), new.notes.as_deref());
    if !d.is_empty() {
        let action = if d.only_field("État") { "state_changed" } else { "updated" };
        trace_project(&tx, me, &new, action, d.into_vec())?;
    }
    commit(tx)?;
    Ok(new)
}

pub fn archive_project(conn: &mut Connection, me: &AuthUser, id: Id, archived: bool) -> Result<Project> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let p = xpeperience_store::projects::get(&tx, id)?.ok_or(DomainError::NotFound("Projet"))?;
    xpeperience_store::projects::set_archived(&tx, id, archived)?;
    trace_project(&tx, me, &p, if archived { "archived" } else { "restored" }, Vec::new())?;
    commit(tx)?;
    xpeperience_store::projects::get(conn, id)?.ok_or(DomainError::NotFound("Projet"))
}

pub fn delete_project(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Projects, Action::Delete)?;
    let tx = begin(conn)?;
    let p = xpeperience_store::projects::get(&tx, id)?.ok_or(DomainError::NotFound("Projet"))?;
    xpeperience_store::projects::delete(&tx, id)?;
    trace(&tx, me, Trace { entity: EntityKind::Project, id, label: &p.title, link: None }, "deleted", &[])?;
    commit(tx)
}

// ───────────────────────────── Prestations ─────────────────────────────

fn get_delivery(conn: &Connection, id: Id) -> Result<Delivery> {
    xpeperience_store::projects::delivery(conn, id)?.ok_or(DomainError::NotFound("Prestation"))
}

pub fn deliveries(conn: &Connection, me: &AuthUser, f: &DeliveryFilter) -> Result<Vec<Delivery>> {
    me.require(Area::Projects, Action::Read)?;
    let mut list = xpeperience_store::projects::deliveries(conn, f)?;
    list.iter_mut().for_each(|d| hide_delivery_costs(me, d));
    Ok(list)
}

fn clean_delivery(conn: &Connection, refs: &Refs, i: DeliveryInput) -> Result<(DeliveryInput, String)> {
    let settings = xpeperience_store::settings::load(conn)?;
    let i = DeliveryInput {
        title: clean(i.title),
        start_date: clean(i.start_date),
        end_date: clean(i.end_date),
        notes: clean(i.notes),
        ..i
    };
    let mut errors = Errors::new();
    if i.start_date.is_none() {
        errors.add("start_date", "Indiquez la date de début de la prestation.");
    }
    check_date(&mut errors, "start_date", i.start_date.as_deref());
    check_date(&mut errors, "end_date", i.end_date.as_deref());
    check_range(&mut errors, "end_date", i.start_date.as_deref(), i.end_date.as_deref());
    check_amount(&mut errors, "daily_rate", i.daily_rate);
    check_amount(&mut errors, "sold_days", i.sold_days);
    check_amount(&mut errors, "daily_cost", i.daily_cost);
    refs.check(&mut errors, "type_id", RefList::DeliveryType, i.type_id);
    if let Some(rid) = i.resource_id
        && xpeperience_store::resources::get(conn, rid)?.is_none()
    {
        errors.add("resource_id", "Ressource introuvable.");
    }
    check_required(
        &mut errors,
        &settings,
        "delivery",
        &[
            ("resource_id", i.resource_id.is_some()),
            ("end_date", i.end_date.is_some()),
            ("daily_rate", i.daily_rate.is_some()),
            ("sold_days", i.sold_days.is_some()),
        ],
    );
    errors.finish()?;
    let start = i.start_date.clone().unwrap_or_default();
    Ok((i, start))
}

fn delivery_label(d: &Delivery) -> String {
    match &d.resource {
        Some(r) => format!("{} — {}", r.name, d.project.name),
        None => d.title.clone().unwrap_or_else(|| format!("Prestation — {}", d.project.name)),
    }
}

fn trace_delivery(conn: &Connection, me: &AuthUser, d: &Delivery, action: &str, changes: Vec<xpeperience_domain::model::FieldChange>) -> Result<()> {
    let t = Trace { entity: EntityKind::Delivery, id: d.id, label: &delivery_label(d), link: Some((EntityKind::Project, d.project.id)) };
    trace(conn, me, t, action, &changes)
}

pub fn create_delivery(conn: &mut Connection, me: &AuthUser, project_id: Id, input: DeliveryInput) -> Result<Delivery> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let project = xpeperience_store::projects::get(&tx, project_id)?.ok_or(DomainError::NotFound("Projet"))?;
    if project.archived {
        return Err(DomainError::rule("Ce projet est archivé : restaurez-le avant d'ajouter une prestation."));
    }
    let refs = Refs::load(&tx)?;
    let (mut i, start) = clean_delivery(&tx, &refs, input)?;
    if !can_edit_costs(me) || i.daily_cost.is_none() {
        // Le CJM est repris de la ressource quand il est connu ; il peut rester vide.
        i.daily_cost = match i.resource_id {
            Some(rid) => xpeperience_store::resources::get(&tx, rid)?.and_then(|r| r.daily_cost),
            None => None,
        };
    }
    let id = xpeperience_store::projects::insert_delivery(&tx, project_id, &i, &start, None)?;
    recompute_dates(&tx, project_id)?;
    let mut d = get_delivery(&tx, id)?;
    trace_delivery(&tx, me, &d, "created", Vec::new())?;
    commit(tx)?;
    hide_delivery_costs(me, &mut d);
    Ok(d)
}

pub fn update_delivery(conn: &mut Connection, me: &AuthUser, id: Id, input: DeliveryInput) -> Result<Delivery> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let old = get_delivery(&tx, id)?;
    let refs = Refs::load(&tx)?;
    let (mut i, start) = clean_delivery(&tx, &refs, input)?;
    if !can_edit_costs(me) {
        i.daily_cost = old.daily_cost;
    }
    if let Some(first) = old.extensions.iter().map(|e| e.start_date.as_str()).min()
        && i.end_date.as_deref().is_none_or(|e| e >= first)
    {
        return Err(DomainError::invalid(
            "end_date",
            "La fin initiale doit rester avant le début des prolongations. Modifiez plutôt les prolongations.",
        ));
    }
    xpeperience_store::projects::update_delivery(&tx, id, &i, &start)?;
    recompute_dates(&tx, old.project.id)?;
    let mut new = get_delivery(&tx, id)?;

    let mut d = Diff::new();
    d.named("Ressource", old.resource.as_ref(), new.resource.as_ref());
    d.reference("Type", &refs, old.type_id, new.type_id);
    d.text("Intitulé", old.title.as_deref(), new.title.as_deref());
    d.text("Début", Some(&old.start_date), Some(&new.start_date));
    d.text("Fin", old.end_date.as_deref(), new.end_date.as_deref());
    d.num("Tarif journalier", old.daily_rate, new.daily_rate);
    d.num("Jours vendus", old.sold_days, new.sold_days);
    if can_edit_costs(me) {
        d.num("CJM", old.daily_cost, new.daily_cost);
    }
    d.text("Notes", old.notes.as_deref(), new.notes.as_deref());
    if !d.is_empty() {
        trace_delivery(&tx, me, &new, "updated", d.into_vec())?;
    }
    commit(tx)?;
    hide_delivery_costs(me, &mut new);
    Ok(new)
}

pub fn delete_delivery(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let d = get_delivery(&tx, id)?;
    xpeperience_store::projects::delete_delivery(&tx, id)?;
    recompute_dates(&tx, d.project.id)?;
    trace_delivery(&tx, me, &d, "deleted", Vec::new())?;
    commit(tx)
}

// ──────────────────────────── Prolongations ────────────────────────────

fn clean_extension(d: &Delivery, i: &ExtensionInput, exclude: Option<Id>) -> Result<(ExtensionStatus, String, String)> {
    let start = clean(i.start_date.clone());
    let end = clean(i.end_date.clone());
    let mut errors = Errors::new();
    if start.is_none() {
        errors.add("start_date", "Indiquez le début de la prolongation.");
    }
    if end.is_none() {
        errors.add("end_date", "Indiquez la fin de la prolongation.");
    }
    check_date(&mut errors, "start_date", start.as_deref());
    check_date(&mut errors, "end_date", end.as_deref());
    check_amount(&mut errors, "days", i.days);
    check_amount(&mut errors, "daily_rate", i.daily_rate);
    errors.finish()?;
    let (start, end) = (start.unwrap_or_default(), end.unwrap_or_default());
    let mut errors = Errors::new();
    check_extension(&mut errors, d.end_date.as_deref(), &start, &end);
    if d.extensions.iter().any(|e| Some(e.id) != exclude && !(end < e.start_date || start > e.end_date)) {
        errors.add("start_date", "Cette période chevauche une autre prolongation.");
    }
    errors.finish()?;
    Ok((i.status.unwrap_or(ExtensionStatus::Forecast), start, end))
}

fn extension_change(status: ExtensionStatus, start: &str, end: &str) -> xpeperience_domain::model::FieldChange {
    xpeperience_domain::model::FieldChange {
        field: format!("Prolongation {}", status.label()),
        from: None,
        to: Some(format!("{start} → {end}")),
    }
}

pub fn create_extension(conn: &mut Connection, me: &AuthUser, delivery_id: Id, input: ExtensionInput) -> Result<Delivery> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let d = get_delivery(&tx, delivery_id)?;
    let (status, start, end) = clean_extension(&d, &input, None)?;
    let i = ExtensionInput { notes: clean(input.notes.clone()), ..input };
    xpeperience_store::projects::insert_extension(&tx, delivery_id, status, &start, &end, &i)?;
    recompute_dates(&tx, d.project.id)?;
    trace_delivery(&tx, me, &d, "updated", vec![extension_change(status, &start, &end)])?;
    let mut d = get_delivery(&tx, delivery_id)?;
    commit(tx)?;
    hide_delivery_costs(me, &mut d);
    Ok(d)
}

pub fn update_extension(conn: &mut Connection, me: &AuthUser, id: Id, input: ExtensionInput) -> Result<Delivery> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let ext = xpeperience_store::projects::extension(&tx, id)?.ok_or(DomainError::NotFound("Prolongation"))?;
    let d = get_delivery(&tx, ext.delivery_id)?;
    let (status, start, end) = clean_extension(&d, &input, Some(id))?;
    let i = ExtensionInput { notes: clean(input.notes.clone()), ..input };
    xpeperience_store::projects::update_extension(&tx, id, status, &start, &end, &i)?;
    recompute_dates(&tx, d.project.id)?;
    if (ext.status, ext.start_date.as_str(), ext.end_date.as_str()) != (status, start.as_str(), end.as_str()) {
        trace_delivery(&tx, me, &d, "updated", vec![extension_change(status, &start, &end)])?;
    }
    let mut d = get_delivery(&tx, ext.delivery_id)?;
    commit(tx)?;
    hide_delivery_costs(me, &mut d);
    Ok(d)
}

pub fn delete_extension(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<Delivery> {
    me.require(Area::Projects, Action::Update)?;
    let tx = begin(conn)?;
    let ext = xpeperience_store::projects::extension(&tx, id)?.ok_or(DomainError::NotFound("Prolongation"))?;
    let d = get_delivery(&tx, ext.delivery_id)?;
    xpeperience_store::projects::delete_extension(&tx, id)?;
    recompute_dates(&tx, d.project.id)?;
    let change = xpeperience_domain::model::FieldChange {
        field: format!("Prolongation {}", ext.status.label()),
        from: Some(format!("{} → {}", ext.start_date, ext.end_date)),
        to: None,
    };
    trace_delivery(&tx, me, &d, "updated", vec![change])?;
    let mut d = get_delivery(&tx, ext.delivery_id)?;
    commit(tx)?;
    hide_delivery_costs(me, &mut d);
    Ok(d)
}
