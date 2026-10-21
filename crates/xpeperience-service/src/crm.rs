//! CRM : sociétés, contacts et activités commerciales.

use xpeperience_domain::error::Errors;
use xpeperience_domain::model::{
    Activity, ActivityFilter, ActivityInput, ActivityKind, Company, CompanyFilter, CompanyInput, Contact, ContactFilter,
    ContactInput, EntityKind,
};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::refs::RefList;
use xpeperience_domain::rules::{check_date, check_email, check_required};
use xpeperience_domain::text::{clean, clean_required, linkedin_key};
use xpeperience_domain::{DomainError, Id, Result};
use xpeperience_store::Connection;
use xpeperience_store::contacts::{DuplicateQuery, Origin};
use serde_json::json;

use crate::AuthUser;
use crate::support::{Diff, Refs, Trace, begin, check_user, commit, today_plus, trace};

// ───────────────────────────── Sociétés ─────────────────────────────

pub fn companies(conn: &Connection, me: &AuthUser, f: &CompanyFilter) -> Result<Vec<Company>> {
    me.require(Area::Crm, Action::Read)?;
    Ok(xpeperience_store::companies::list(conn, f)?)
}

pub fn company(conn: &Connection, me: &AuthUser, id: Id) -> Result<Company> {
    me.require(Area::Crm, Action::Read)?;
    xpeperience_store::companies::get(conn, id)?.ok_or(DomainError::NotFound("Société"))
}

fn clean_company(conn: &Connection, refs: &Refs, i: CompanyInput) -> Result<CompanyInput> {
    let settings = xpeperience_store::settings::load(conn)?;
    let i = CompanyInput {
        name: clean_required(&i.name),
        type_id: i.type_id,
        sector_id: i.sector_id,
        website: clean(i.website),
        phone: clean(i.phone),
        email: clean(i.email),
        address: clean(i.address),
        postal_code: clean(i.postal_code),
        city: clean(i.city),
        country: clean(i.country),
        notes: clean(i.notes),
        owner_id: i.owner_id,
    };
    let mut errors = Errors::new();
    if i.name.is_empty() {
        errors.add("name", "Indiquez le nom de la société.");
    }
    check_email(&mut errors, "email", i.email.as_deref());
    refs.check(&mut errors, "type_id", RefList::CompanyType, i.type_id);
    refs.check(&mut errors, "sector_id", RefList::Sector, i.sector_id);
    check_user(conn, &mut errors, "owner_id", i.owner_id)?;
    check_required(
        &mut errors,
        &settings,
        "company",
        &[
            ("type_id", i.type_id.is_some()),
            ("sector_id", i.sector_id.is_some()),
            ("owner_id", i.owner_id.is_some()),
            ("phone", i.phone.is_some()),
            ("email", i.email.is_some()),
            ("city", i.city.is_some()),
        ],
    );
    errors.finish()?;
    Ok(i)
}

fn ensure_unique_company(conn: &Connection, name: &str, exclude: Option<Id>) -> Result<()> {
    if let Some(existing) = xpeperience_store::companies::find_by_name(conn, name, exclude)? {
        return Err(DomainError::conflict(
            format!("La société « {} » existe déjà.", existing.name),
            Some(json!({ "entity": "company", "id": existing.id, "name": existing.name })),
        ));
    }
    Ok(())
}

pub(crate) fn insert_company(conn: &Connection, me: &AuthUser, input: CompanyInput) -> Result<Company> {
    let refs = Refs::load(conn)?;
    let mut input = clean_company(conn, &refs, input)?;
    ensure_unique_company(conn, &input.name, None)?;
    input.owner_id = input.owner_id.or(Some(me.id));
    let id = xpeperience_store::companies::insert(conn, &input, me.id)?;
    trace(conn, me, Trace { entity: EntityKind::Company, id, label: &input.name, link: None }, "created", &[])?;
    xpeperience_store::companies::get(conn, id)?.ok_or(DomainError::NotFound("Société"))
}

pub fn create_company(conn: &mut Connection, me: &AuthUser, input: CompanyInput) -> Result<Company> {
    me.require(Area::Crm, Action::Create)?;
    let tx = begin(conn)?;
    let company = insert_company(&tx, me, input)?;
    commit(tx)?;
    Ok(company)
}

pub fn update_company(conn: &mut Connection, me: &AuthUser, id: Id, input: CompanyInput) -> Result<Company> {
    me.require(Area::Crm, Action::Update)?;
    let tx = begin(conn)?;
    let old = xpeperience_store::companies::get(&tx, id)?.ok_or(DomainError::NotFound("Société"))?;
    let refs = Refs::load(&tx)?;
    let input = clean_company(&tx, &refs, input)?;
    ensure_unique_company(&tx, &input.name, Some(id))?;
    xpeperience_store::companies::update(&tx, id, &input)?;
    let new = xpeperience_store::companies::get(&tx, id)?.ok_or(DomainError::NotFound("Société"))?;

    let mut d = Diff::new();
    d.text("Nom", Some(&old.name), Some(&new.name));
    d.reference("Type", &refs, old.type_id, new.type_id);
    d.reference("Secteur", &refs, old.sector_id, new.sector_id);
    d.text("Site web", old.website.as_deref(), new.website.as_deref());
    d.text("Téléphone", old.phone.as_deref(), new.phone.as_deref());
    d.text("E-mail", old.email.as_deref(), new.email.as_deref());
    d.text("Adresse", old.address.as_deref(), new.address.as_deref());
    d.text("Code postal", old.postal_code.as_deref(), new.postal_code.as_deref());
    d.text("Ville", old.city.as_deref(), new.city.as_deref());
    d.text("Pays", old.country.as_deref(), new.country.as_deref());
    d.text("Notes", old.notes.as_deref(), new.notes.as_deref());
    d.named("Responsable commercial", old.owner.as_ref(), new.owner.as_ref());
    if !d.is_empty() {
        trace(&tx, me, Trace { entity: EntityKind::Company, id, label: &new.name, link: None }, "updated", &d.into_vec())?;
    }
    commit(tx)?;
    Ok(new)
}

pub fn archive_company(conn: &mut Connection, me: &AuthUser, id: Id, archived: bool) -> Result<Company> {
    me.require(Area::Crm, Action::Update)?;
    let tx = begin(conn)?;
    let c = xpeperience_store::companies::get(&tx, id)?.ok_or(DomainError::NotFound("Société"))?;
    xpeperience_store::companies::set_archived(&tx, id, archived)?;
    let action = if archived { "archived" } else { "restored" };
    trace(&tx, me, Trace { entity: EntityKind::Company, id, label: &c.name, link: None }, action, &[])?;
    commit(tx)?;
    xpeperience_store::companies::get(conn, id)?.ok_or(DomainError::NotFound("Société"))
}

pub fn delete_company(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Crm, Action::Delete)?;
    let tx = begin(conn)?;
    let c = xpeperience_store::companies::get(&tx, id)?.ok_or(DomainError::NotFound("Société"))?;
    let links = xpeperience_store::companies::links(&tx, id)?;
    if links > 0 {
        return Err(DomainError::rule(format!(
            "« {} » est liée à {links} contact(s), besoin(s) ou projet(s). Archivez-la plutôt pour conserver l'historique.",
            c.name
        )));
    }
    xpeperience_store::companies::delete(&tx, id)?;
    trace(&tx, me, Trace { entity: EntityKind::Company, id, label: &c.name, link: None }, "deleted", &[])?;
    commit(tx)
}

// ───────────────────────────── Contacts ─────────────────────────────

pub fn contacts(conn: &Connection, me: &AuthUser, f: &ContactFilter) -> Result<Vec<Contact>> {
    me.require(Area::Crm, Action::Read)?;
    Ok(xpeperience_store::contacts::list(conn, f)?)
}

pub fn contact(conn: &Connection, me: &AuthUser, id: Id) -> Result<Contact> {
    me.require(Area::Crm, Action::Read)?;
    xpeperience_store::contacts::get(conn, id)?.ok_or(DomainError::NotFound("Contact"))
}

pub(crate) fn clean_contact(conn: &Connection, i: ContactInput) -> Result<ContactInput> {
    let settings = xpeperience_store::settings::load(conn)?;
    let i = ContactInput {
        company_id: i.company_id,
        first_name: clean(i.first_name),
        last_name: clean_required(&i.last_name),
        job_title: clean(i.job_title),
        email: clean(i.email).map(|e| e.to_lowercase()),
        phone: clean(i.phone),
        mobile: clean(i.mobile),
        linkedin_url: clean(i.linkedin_url),
        location: clean(i.location),
        notes: clean(i.notes),
        owner_id: i.owner_id,
        force: i.force,
    };
    let mut errors = Errors::new();
    if i.last_name.is_empty() {
        errors.add("last_name", "Indiquez au moins le nom du contact.");
    }
    check_email(&mut errors, "email", i.email.as_deref());
    if i.linkedin_url.as_deref().is_some_and(|u| linkedin_key(u).is_none()) {
        errors.add("linkedin_url", "Lien de profil LinkedIn invalide (linkedin.com/in/…).");
    }
    if let Some(cid) = i.company_id
        && xpeperience_store::companies::get(conn, cid)?.is_none()
    {
        errors.add("company_id", "Société introuvable.");
    }
    check_user(conn, &mut errors, "owner_id", i.owner_id)?;
    check_required(
        &mut errors,
        &settings,
        "contact",
        &[
            ("first_name", i.first_name.is_some()),
            ("company_id", i.company_id.is_some()),
            ("job_title", i.job_title.is_some()),
            ("email", i.email.is_some()),
            ("phone", i.phone.is_some() || i.mobile.is_some()),
            ("owner_id", i.owner_id.is_some()),
        ],
    );
    errors.finish()?;
    Ok(i)
}

pub(crate) fn find_duplicates(
    conn: &Connection,
    i: &ContactInput,
    exclude: Option<Id>,
) -> Result<Vec<xpeperience_domain::model::DuplicateContact>> {
    Ok(xpeperience_store::contacts::duplicates(
        conn,
        &DuplicateQuery {
            first_name: i.first_name.as_deref(),
            last_name: &i.last_name,
            email: i.email.as_deref(),
            linkedin_url: i.linkedin_url.as_deref(),
            company_id: i.company_id,
            exclude,
        },
    )?)
}

/// Crée un contact après les vérifications de doublons (sauf confirmation explicite).
pub(crate) fn insert_contact(conn: &Connection, me: &AuthUser, input: ContactInput, origin: Origin) -> Result<Contact> {
    let mut input = clean_contact(conn, input)?;
    if !input.force {
        let duplicates = find_duplicates(conn, &input, None)?;
        if !duplicates.is_empty() {
            return Err(DomainError::conflict(
                "Un contact similaire existe déjà.",
                Some(json!({ "entity": "contact", "duplicates": duplicates })),
            ));
        }
    }
    input.owner_id = input.owner_id.or(Some(me.id));
    let id = xpeperience_store::contacts::insert(conn, &input, origin, me.id)?;
    let contact = xpeperience_store::contacts::get(conn, id)?.ok_or(DomainError::NotFound("Contact"))?;
    let action = if contact.source == "linkedin" { "imported" } else { "created" };
    let link = contact.company.as_ref().map(|c| (EntityKind::Company, c.id));
    trace(conn, me, Trace { entity: EntityKind::Contact, id, label: &contact.full_name(), link }, action, &[])?;
    Ok(contact)
}

pub fn create_contact(conn: &mut Connection, me: &AuthUser, input: ContactInput) -> Result<Contact> {
    me.require(Area::Crm, Action::Create)?;
    let tx = begin(conn)?;
    let contact = insert_contact(&tx, me, input, Origin { source: "manual", imported_at: None })?;
    commit(tx)?;
    Ok(contact)
}

pub(crate) fn apply_contact_update(conn: &Connection, me: &AuthUser, id: Id, input: ContactInput, action: &str) -> Result<Contact> {
    let old = xpeperience_store::contacts::get(conn, id)?.ok_or(DomainError::NotFound("Contact"))?;
    let input = clean_contact(conn, input)?;
    xpeperience_store::contacts::update(conn, id, &input)?;
    let new = xpeperience_store::contacts::get(conn, id)?.ok_or(DomainError::NotFound("Contact"))?;

    let mut d = Diff::new();
    d.text("Prénom", old.first_name.as_deref(), new.first_name.as_deref());
    d.text("Nom", Some(&old.last_name), Some(&new.last_name));
    d.named("Société", old.company.as_ref(), new.company.as_ref());
    d.text("Poste", old.job_title.as_deref(), new.job_title.as_deref());
    d.text("E-mail", old.email.as_deref(), new.email.as_deref());
    d.text("Téléphone", old.phone.as_deref(), new.phone.as_deref());
    d.text("Mobile", old.mobile.as_deref(), new.mobile.as_deref());
    d.text("LinkedIn", old.linkedin_url.as_deref(), new.linkedin_url.as_deref());
    d.text("Localisation", old.location.as_deref(), new.location.as_deref());
    d.text("Notes", old.notes.as_deref(), new.notes.as_deref());
    d.named("Responsable commercial", old.owner.as_ref(), new.owner.as_ref());
    if !d.is_empty() || action != "updated" {
        let link = new.company.as_ref().map(|c| (EntityKind::Company, c.id));
        trace(conn, me, Trace { entity: EntityKind::Contact, id, label: &new.full_name(), link }, action, &d.into_vec())?;
    }
    Ok(new)
}

pub fn update_contact(conn: &mut Connection, me: &AuthUser, id: Id, input: ContactInput) -> Result<Contact> {
    me.require(Area::Crm, Action::Update)?;
    let tx = begin(conn)?;
    let contact = apply_contact_update(&tx, me, id, input, "updated")?;
    commit(tx)?;
    Ok(contact)
}

pub fn archive_contact(conn: &mut Connection, me: &AuthUser, id: Id, archived: bool) -> Result<Contact> {
    me.require(Area::Crm, Action::Update)?;
    let tx = begin(conn)?;
    let c = xpeperience_store::contacts::get(&tx, id)?.ok_or(DomainError::NotFound("Contact"))?;
    xpeperience_store::contacts::set_archived(&tx, id, archived)?;
    let link = c.company.as_ref().map(|co| (EntityKind::Company, co.id));
    let action = if archived { "archived" } else { "restored" };
    trace(&tx, me, Trace { entity: EntityKind::Contact, id, label: &c.full_name(), link }, action, &[])?;
    commit(tx)?;
    xpeperience_store::contacts::get(conn, id)?.ok_or(DomainError::NotFound("Contact"))
}

pub fn delete_contact(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Crm, Action::Delete)?;
    let tx = begin(conn)?;
    let c = xpeperience_store::contacts::get(&tx, id)?.ok_or(DomainError::NotFound("Contact"))?;
    let links = xpeperience_store::contacts::links(&tx, id)?;
    if links > 0 {
        return Err(DomainError::rule(format!(
            "{} est lié(e) à {links} besoin(s) ou projet(s). Archivez ce contact plutôt pour conserver l'historique.",
            c.full_name()
        )));
    }
    xpeperience_store::contacts::delete(&tx, id)?;
    trace(&tx, me, Trace { entity: EntityKind::Contact, id, label: &c.full_name(), link: None }, "deleted", &[])?;
    commit(tx)
}

// ──────────────────────────── Activités ────────────────────────────

/// Domaine de droits d'une activité : celui de l'objet le plus précis auquel elle est liée.
fn activity_area(positioning: Option<Id>, need: Option<Id>, project: Option<Id>, resource: Option<Id>) -> Area {
    if positioning.is_some() {
        Area::Positionings
    } else if need.is_some() {
        Area::Needs
    } else if project.is_some() {
        Area::Projects
    } else if resource.is_some() {
        Area::Resources
    } else {
        Area::Crm
    }
}

fn can_write_activity(me: &AuthUser, area: Area) -> bool {
    me.can(area, Action::Create) || me.can(area, Action::Update)
}

pub fn activities(conn: &Connection, me: &AuthUser, f: &ActivityFilter) -> Result<Vec<Activity>> {
    let own_reminders = f.reminders == Some(true) && f.owner_id == Some(me.id);
    if !own_reminders {
        me.require(activity_area(f.positioning_id, f.need_id, f.project_id, f.resource_id), Action::Read)?;
    }
    Ok(xpeperience_store::activities::list(conn, f)?)
}

pub fn create_activity(conn: &mut Connection, me: &AuthUser, input: ActivityInput) -> Result<Activity> {
    let tx = begin(conn)?;
    let mut i = input;
    let mut errors = Errors::new();

    // Complète les liens à partir de l'objet le plus précis.
    if let Some(pid) = i.positioning_id {
        match xpeperience_store::positionings::get(&tx, pid)? {
            Some(p) => {
                i.need_id = Some(p.need.id);
                i.resource_id = Some(p.resource.id);
            }
            None => errors.add("positioning_id", "Positionnement introuvable."),
        }
    }
    if let Some(nid) = i.need_id {
        match xpeperience_store::needs::get(&tx, nid)? {
            Some(n) => {
                i.contact_id = i.contact_id.or(n.contact.map(|c| c.id));
                i.company_id = i.company_id.or(n.company.map(|c| c.id));
            }
            None => errors.add("need_id", "Besoin introuvable."),
        }
    }
    if let Some(pid) = i.project_id {
        match xpeperience_store::projects::get(&tx, pid)? {
            Some(p) => i.company_id = i.company_id.or(p.company.map(|c| c.id)),
            None => errors.add("project_id", "Projet introuvable."),
        }
    }
    if let Some(cid) = i.contact_id {
        match xpeperience_store::contacts::get(&tx, cid)? {
            Some(c) => i.company_id = i.company_id.or(c.company.map(|co| co.id)),
            None => errors.add("contact_id", "Contact introuvable."),
        }
    }
    if let Some(cid) = i.company_id
        && xpeperience_store::companies::get(&tx, cid)?.is_none()
    {
        errors.add("company_id", "Société introuvable.");
    }
    if let Some(rid) = i.resource_id
        && xpeperience_store::resources::get(&tx, rid)?.is_none()
    {
        errors.add("resource_id", "Ressource introuvable.");
    }
    if [i.company_id, i.contact_id, i.need_id, i.resource_id, i.positioning_id, i.project_id].iter().all(Option::is_none) {
        errors.add("kind", "Rattachez l'activité à une fiche.");
    }
    errors.finish()?;

    let area = activity_area(i.positioning_id, i.need_id, i.project_id, i.resource_id);
    if !can_write_activity(me, area) {
        return Err(DomainError::Forbidden);
    }

    let settings = xpeperience_store::settings::load(&tx)?;
    let kind = i.kind.unwrap_or(ActivityKind::Note);
    let comment = i.comment.trim().to_string();
    let due = clean(i.due_date.clone()).or_else(|| {
        (kind == ActivityKind::FollowUp).then(|| today_plus(settings.reminders.default_follow_up_days))
    });
    let occurred = clean(i.occurred_at.clone()).unwrap_or_else(xpeperience_store::now);
    let mut errors = Errors::new();
    if kind == ActivityKind::Note && comment.is_empty() {
        errors.add("comment", "Écrivez le contenu de la note.");
    }
    check_date(&mut errors, "due_date", due.as_deref());
    check_user(&tx, &mut errors, "owner_id", i.owner_id)?;
    errors.finish()?;

    let id = xpeperience_store::activities::insert(
        &tx,
        &xpeperience_store::activities::NewActivity {
            kind,
            occurred_at: &occurred,
            comment: &comment,
            due_date: due.as_deref(),
            company_id: i.company_id,
            contact_id: i.contact_id,
            need_id: i.need_id,
            resource_id: i.resource_id,
            positioning_id: i.positioning_id,
            project_id: i.project_id,
            owner_id: i.owner_id.or(Some(me.id)),
        },
        me.id,
    )?;
    commit(tx)?;
    xpeperience_store::activities::get(conn, id)?.ok_or(DomainError::NotFound("Activité"))
}

fn editable_activity(conn: &Connection, me: &AuthUser, id: Id, action: Action) -> Result<Activity> {
    let a = xpeperience_store::activities::get(conn, id)?.ok_or(DomainError::NotFound("Activité"))?;
    let area = activity_area(a.positioning_id, a.need.as_ref().map(|n| n.id), a.project.as_ref().map(|p| p.id), a.resource.as_ref().map(|r| r.id));
    let mine = a.owner.as_ref().is_some_and(|o| o.id == me.id);
    if !mine && !me.can(area, action) {
        return Err(DomainError::Forbidden);
    }
    Ok(a)
}

pub fn update_activity(conn: &Connection, me: &AuthUser, id: Id, input: ActivityInput) -> Result<Activity> {
    let a = editable_activity(conn, me, id, Action::Update)?;
    let kind = input.kind.unwrap_or(a.kind);
    let comment = input.comment.trim().to_string();
    let due = clean(input.due_date);
    let occurred = clean(input.occurred_at).unwrap_or(a.occurred_at);
    let mut errors = Errors::new();
    if kind == ActivityKind::Note && comment.is_empty() {
        errors.add("comment", "Écrivez le contenu de la note.");
    }
    check_date(&mut errors, "due_date", due.as_deref());
    check_user(conn, &mut errors, "owner_id", input.owner_id)?;
    errors.finish()?;
    xpeperience_store::activities::update(
        conn,
        id,
        &xpeperience_store::activities::NewActivity {
            kind,
            occurred_at: &occurred,
            comment: &comment,
            due_date: due.as_deref(),
            company_id: None,
            contact_id: None,
            need_id: None,
            resource_id: None,
            positioning_id: None,
            project_id: None,
            owner_id: input.owner_id.or(a.owner.map(|o| o.id)),
        },
    )?;
    xpeperience_store::activities::get(conn, id)?.ok_or(DomainError::NotFound("Activité"))
}

pub fn set_activity_done(conn: &Connection, me: &AuthUser, id: Id, done: bool) -> Result<Activity> {
    editable_activity(conn, me, id, Action::Update)?;
    xpeperience_store::activities::set_done(conn, id, done)?;
    xpeperience_store::activities::get(conn, id)?.ok_or(DomainError::NotFound("Activité"))
}

pub fn delete_activity(conn: &Connection, me: &AuthUser, id: Id) -> Result<()> {
    editable_activity(conn, me, id, Action::Delete)?;
    xpeperience_store::activities::delete(conn, id)?;
    Ok(())
}
