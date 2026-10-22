//! Positionnements : associer des candidats ou consultants à un besoin.

use xpeperience_domain::error::Errors;
use xpeperience_domain::matching::{Criteria, score};
use xpeperience_domain::model::{
    ConvertInput, DeliveryInput, EntityKind, Named, Positioning, PositioningFilter, PositioningInput, PositioningSaved,
    Project, ResourceFilter, Suggestion,
};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::refs::RefList;
use xpeperience_domain::rules::{check_amount, check_date, check_range};
use xpeperience_domain::text::clean;
use xpeperience_domain::{DomainError, Id, Result};
use xpeperience_store::Connection;

use crate::AuthUser;
use crate::needs::{mark_won, project_from_need};
use crate::projects::recompute_dates;
use crate::support::{
    Diff, Refs, Trace, begin, can_edit_costs, commit, hide_positioning_costs, hide_resource_costs, trace,
};

fn label(p: &Positioning) -> String {
    format!("{} — {}", p.resource.name, p.need.name)
}

fn get(conn: &Connection, id: Id) -> Result<Positioning> {
    xpeperience_store::positionings::get(conn, id)?.ok_or(DomainError::NotFound("Positionnement"))
}

pub fn positionings(conn: &Connection, me: &AuthUser, f: &PositioningFilter) -> Result<Vec<Positioning>> {
    me.require(Area::Positionings, Action::Read)?;
    let mut list = xpeperience_store::positionings::list(conn, f)?;
    list.iter_mut().for_each(|p| hide_positioning_costs(me, p));
    Ok(list)
}

pub fn positioning(conn: &Connection, me: &AuthUser, id: Id) -> Result<Positioning> {
    me.require(Area::Positionings, Action::Read)?;
    let mut p = get(conn, id)?;
    hide_positioning_costs(me, &mut p);
    Ok(p)
}

fn check_values(refs: &Refs, i: &PositioningInput) -> Result<()> {
    let mut errors = Errors::new();
    refs.check(&mut errors, "state_id", RefList::PositioningState, i.state_id);
    check_date(&mut errors, "start_date", i.start_date.as_deref());
    check_date(&mut errors, "end_date", i.end_date.as_deref());
    check_range(&mut errors, "end_date", i.start_date.as_deref(), i.end_date.as_deref());
    check_amount(&mut errors, "daily_rate", i.daily_rate);
    check_amount(&mut errors, "days", i.days);
    check_amount(&mut errors, "daily_cost", i.daily_cost);
    errors.finish()
}

pub fn create_positioning(conn: &mut Connection, me: &AuthUser, input: PositioningInput) -> Result<PositioningSaved> {
    me.require(Area::Positionings, Action::Create)?;
    let tx = begin(conn)?;
    let refs = Refs::load(&tx)?;
    let mut i = PositioningInput { start_date: clean(input.start_date), end_date: clean(input.end_date), notes: clean(input.notes), ..input };

    let mut errors = Errors::new();
    let need = match i.need_id {
        Some(id) => xpeperience_store::needs::get(&tx, id)?,
        None => None,
    };
    let resource = match i.resource_id {
        Some(id) => xpeperience_store::resources::get(&tx, id)?,
        None => None,
    };
    if need.is_none() {
        errors.add("need_id", "Besoin introuvable.");
    }
    match &resource {
        None => errors.add("resource_id", "Choisissez une ressource."),
        Some(r) if r.archived => errors.add("resource_id", "Cette ressource est archivée."),
        _ => {}
    }
    errors.finish()?;
    let (need, resource) = (need.unwrap(), resource.unwrap());

    if let Some(existing) = xpeperience_store::positionings::exists(&tx, need.id, resource.id)? {
        return Err(DomainError::conflict(
            format!("{} est déjà positionné(e) sur ce besoin.", resource.full_name()),
            Some(serde_json::json!({ "entity": "positioning", "id": existing })),
        ));
    }

    // Valeurs reprises du besoin et de la ressource quand elles ne sont pas saisies.
    i.start_date = i.start_date.or(need.start_date.clone());
    i.days = i.days.or(need.planned_days);
    if !can_edit_costs(me) || i.daily_cost.is_none() {
        i.daily_cost = resource.daily_cost;
    }
    check_values(&refs, &i)?;
    let state_id = match i.state_id {
        Some(s) => s,
        None => refs
            .first(RefList::PositioningState, Some("open"))
            .ok_or_else(|| DomainError::rule("Aucun état « En cours » actif pour les positionnements."))?,
    };
    let id = xpeperience_store::positionings::insert(&tx, need.id, resource.id, state_id, &i, me.id)?;
    let p = get(&tx, id)?;
    trace(&tx, me, Trace { entity: EntityKind::Positioning, id, label: &label(&p), link: Some((EntityKind::Need, need.id)) }, "created", &[])?;
    let saved = after_state_change(&tx, me, &refs, p, None)?;
    commit(tx)?;
    Ok(saved)
}

pub fn update_positioning(conn: &mut Connection, me: &AuthUser, id: Id, input: PositioningInput) -> Result<PositioningSaved> {
    me.require(Area::Positionings, Action::Update)?;
    let tx = begin(conn)?;
    let refs = Refs::load(&tx)?;
    let old = get(&tx, id)?;
    let mut i = PositioningInput { start_date: clean(input.start_date), end_date: clean(input.end_date), notes: clean(input.notes), ..input };
    if !can_edit_costs(me) {
        i.daily_cost = old.daily_cost;
    }
    check_values(&refs, &i)?;
    let state_id = i.state_id.unwrap_or(old.state_id);
    xpeperience_store::positionings::update(&tx, id, state_id, &i)?;
    let new = get(&tx, id)?;

    let mut d = Diff::new();
    d.reference("État", &refs, Some(old.state_id), Some(new.state_id));
    d.text("Début prévu", old.start_date.as_deref(), new.start_date.as_deref());
    d.text("Fin prévue", old.end_date.as_deref(), new.end_date.as_deref());
    d.num("Tarif journalier", old.daily_rate, new.daily_rate);
    d.num("Jours à vendre", old.days, new.days);
    if can_edit_costs(me) {
        d.num("CJM", old.daily_cost, new.daily_cost);
    }
    d.text("Notes", old.notes.as_deref(), new.notes.as_deref());
    if !d.is_empty() {
        let action = if d.only_field("État") { "state_changed" } else { "updated" };
        let t = Trace { entity: EntityKind::Positioning, id, label: &label(&new), link: Some((EntityKind::Need, new.need.id)) };
        trace(&tx, me, t, action, &d.into_vec())?;
    }
    let saved = after_state_change(&tx, me, &refs, new, Some(old.state_id))?;
    commit(tx)?;
    Ok(saved)
}

/// Applique la règle « positionnement gagné » définie dans les réglages.
fn after_state_change(
    conn: &Connection,
    me: &AuthUser,
    refs: &Refs,
    mut p: Positioning,
    old_state: Option<Id>,
) -> Result<PositioningSaved> {
    let became_won = refs.category(Some(p.state_id)) == Some("won") && refs.category(old_state) != Some("won");
    let mut saved = PositioningSaved { positioning: p.clone(), suggest_project: false, created_project: None };
    if !became_won || p.project.is_some() {
        hide_positioning_costs(me, &mut saved.positioning);
        return Ok(saved);
    }
    let settings = xpeperience_store::settings::load(conn)?;
    match settings.business.positioning_won_action.as_str() {
        "auto" if me.can(Area::Projects, Action::Create) => {
            let project = convert_inner(conn, me, refs, &p, None)?;
            p = get(conn, p.id)?;
            saved.positioning = p;
            saved.created_project = Some(project);
        }
        "none" => {}
        _ => saved.suggest_project = me.can(Area::Projects, Action::Create) || me.can(Area::Projects, Action::Update),
    }
    hide_positioning_costs(me, &mut saved.positioning);
    Ok(saved)
}

/// Crée la prestation correspondant au positionnement, dans un nouveau projet ou un projet existant.
fn convert_inner(conn: &Connection, me: &AuthUser, refs: &Refs, p: &Positioning, existing: Option<Id>) -> Result<Named> {
    let need = xpeperience_store::needs::get(conn, p.need.id)?.ok_or(DomainError::NotFound("Besoin"))?;
    let project_id = match existing {
        Some(id) => {
            let project = xpeperience_store::projects::get(conn, id)?.ok_or(DomainError::NotFound("Projet"))?;
            if project.archived {
                return Err(DomainError::rule("Ce projet est archivé : restaurez-le avant d'y ajouter une prestation."));
            }
            if need.project.is_none() {
                xpeperience_store::needs::set_project(conn, need.id, id)?;
            }
            id
        }
        None => project_from_need(conn, me, refs, &need)?,
    };
    let start = p.start_date.clone().or(need.start_date.clone()).unwrap_or_else(xpeperience_store::today);
    let delivery = DeliveryInput {
        resource_id: Some(p.resource.id),
        type_id: None,
        title: None,
        start_date: Some(start.clone()),
        end_date: p.end_date.clone().filter(|e| *e >= start),
        daily_rate: p.daily_rate,
        sold_days: p.days,
        daily_cost: p.daily_cost,
        notes: p.notes.clone(),
    };
    let delivery_id = xpeperience_store::projects::insert_delivery(conn, project_id, &delivery, &start, Some(p.id))?;
    xpeperience_store::positionings::set_project(conn, p.id, project_id)?;
    recompute_dates(conn, project_id)?;

    // Le positionnement converti est forcément gagné.
    if refs.category(Some(p.state_id)) != Some("won")
        && let Some(won) = refs.first(RefList::PositioningState, Some("won"))
    {
        let current = get(conn, p.id)?;
        let i = PositioningInput {
            start_date: current.start_date,
            end_date: current.end_date,
            daily_rate: current.daily_rate,
            days: current.days,
            daily_cost: current.daily_cost,
            notes: current.notes,
            ..Default::default()
        };
        xpeperience_store::positionings::update(conn, p.id, won, &i)?;
    }
    mark_won(conn, me, refs, need.id)?;

    let project = xpeperience_store::projects::get(conn, project_id)?.ok_or(DomainError::NotFound("Projet"))?;
    trace(
        conn,
        me,
        Trace {
            entity: EntityKind::Delivery,
            id: delivery_id,
            label: &format!("{} — {}", p.resource.name, project.title),
            link: Some((EntityKind::Project, project_id)),
        },
        "created",
        &[],
    )?;
    trace(
        conn,
        me,
        Trace { entity: EntityKind::Positioning, id: p.id, label: &label(p), link: Some((EntityKind::Need, p.need.id)) },
        "converted",
        &[xpeperience_domain::model::FieldChange { field: "Projet".into(), from: None, to: Some(project.title.clone()) }],
    )?;
    Ok(Named { id: project.id, name: project.title })
}

/// Crée le projet (ou la prestation dans un projet existant) à partir d'un positionnement.
pub fn convert_positioning(conn: &mut Connection, me: &AuthUser, id: Id, input: ConvertInput) -> Result<Project> {
    me.require(Area::Positionings, Action::Update)?;
    let existing = match input.mode.as_str() {
        "new" => {
            me.require(Area::Projects, Action::Create)?;
            None
        }
        "existing" => {
            me.require(Area::Projects, Action::Update)?;
            Some(input.project_id.ok_or_else(|| DomainError::invalid("project_id", "Choisissez un projet."))?)
        }
        _ => return Err(DomainError::invalid("mode", "Mode inconnu.")),
    };
    let tx = begin(conn)?;
    let refs = Refs::load(&tx)?;
    let p = get(&tx, id)?;
    if let Some(project) = &p.project {
        return Err(DomainError::conflict(
            format!("Ce positionnement a déjà donné lieu au projet « {} ».", project.name),
            Some(serde_json::json!({ "entity": "project", "id": project.id })),
        ));
    }
    let project = convert_inner(&tx, me, &refs, &p, existing)?;
    commit(tx)?;
    xpeperience_store::projects::get(conn, project.id)?.ok_or(DomainError::NotFound("Projet"))
}

pub fn delete_positioning(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Positionings, Action::Delete)?;
    let tx = begin(conn)?;
    let p = get(&tx, id)?;
    xpeperience_store::positionings::delete(&tx, id)?;
    trace(&tx, me, Trace { entity: EntityKind::Positioning, id, label: &label(&p), link: Some((EntityKind::Need, p.need.id)) }, "deleted", &[])?;
    commit(tx)
}

/// Profils suggérés pour un besoin, avec l'explication des critères.
pub fn suggestions(conn: &Connection, me: &AuthUser, need_id: Id) -> Result<Vec<Suggestion>> {
    me.require(Area::Positionings, Action::Read)?;
    me.require(Area::Resources, Action::Read)?;
    let need = xpeperience_store::needs::get(conn, need_id)?.ok_or(DomainError::NotFound("Besoin"))?;
    let already = xpeperience_store::positionings::resource_ids_for_need(conn, need_id)?;
    let today = xpeperience_store::today();
    let criteria = Criteria {
        title: &need.title,
        skills: &need.skills,
        start_date: need.start_date.as_deref(),
        location: need.location.as_deref(),
    };
    let mut out: Vec<Suggestion> = xpeperience_store::resources::list(conn, &ResourceFilter::default())?
        .into_iter()
        .filter(|r| !already.contains(&r.id))
        .map(|r| {
            let (score, reasons) = score(&criteria, &r, &today);
            Suggestion { resource: r, score, reasons }
        })
        .filter(|s| s.reasons.iter().any(|r| r.kind == "skills" && r.positive))
        .collect();
    out.sort_by(|a, b| b.score.cmp(&a.score).then(a.resource.last_name.cmp(&b.resource.last_name)));
    out.truncate(8);
    out.iter_mut().for_each(|s| hide_resource_costs(me, &mut s.resource));
    Ok(out)
}
