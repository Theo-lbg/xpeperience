//! Ressources : candidats et consultants, contrats RH, documents.

use xpeperience_domain::error::Errors;
use xpeperience_domain::model::{
    EntityKind, HrContract, HrContractInput, Resource, ResourceFile, ResourceFilter, ResourceInput, ResourceKind,
};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::refs::RefList;
use xpeperience_domain::rules::{check_amount, check_date, check_email, check_range, check_required};
use xpeperience_domain::text::{clean, clean_required, clean_tags, linkedin_key};
use xpeperience_domain::{DomainError, Id, Result};
use xpeperience_store::Connection;

use crate::AuthUser;
use crate::support::{
    Diff, Refs, Trace, begin, can_edit_costs, check_user, commit, hide_contract_costs, hide_resource_costs, trace,
};

fn get(conn: &Connection, id: Id) -> Result<Resource> {
    xpeperience_store::resources::get(conn, id)?.ok_or(DomainError::NotFound("Ressource"))
}

pub fn resources(conn: &Connection, me: &AuthUser, f: &ResourceFilter) -> Result<Vec<Resource>> {
    me.require(Area::Resources, Action::Read)?;
    let mut list = xpeperience_store::resources::list(conn, f)?;
    list.iter_mut().for_each(|r| hide_resource_costs(me, r));
    Ok(list)
}

pub fn resource(conn: &Connection, me: &AuthUser, id: Id) -> Result<Resource> {
    me.require(Area::Resources, Action::Read)?;
    let mut r = get(conn, id)?;
    hide_resource_costs(me, &mut r);
    Ok(r)
}

fn clean_resource(conn: &Connection, refs: &Refs, i: ResourceInput, exclude: Option<Id>) -> Result<ResourceInput> {
    let settings = xpeperience_store::settings::load(conn)?;
    let i = ResourceInput {
        kind: i.kind,
        first_name: clean_required(&i.first_name),
        last_name: clean_required(&i.last_name),
        title: clean(i.title),
        email: clean(i.email).map(|e| e.to_lowercase()),
        phone: clean(i.phone),
        location: clean(i.location),
        linkedin_url: clean(i.linkedin_url),
        skills: clean_tags(i.skills),
        experience_years: i.experience_years,
        experience: clean(i.experience),
        availability_date: clean(i.availability_date),
        availability_note: clean(i.availability_note),
        contract_type_id: i.contract_type_id,
        daily_cost: i.daily_cost,
        notes: clean(i.notes),
        owner_id: i.owner_id,
    };
    let mut errors = Errors::new();
    if i.first_name.is_empty() {
        errors.add("first_name", "Indiquez le prénom.");
    }
    if i.last_name.is_empty() {
        errors.add("last_name", "Indiquez le nom.");
    }
    check_email(&mut errors, "email", i.email.as_deref());
    if i.linkedin_url.as_deref().is_some_and(|u| linkedin_key(u).is_none()) {
        errors.add("linkedin_url", "Lien de profil LinkedIn invalide (linkedin.com/in/…).");
    }
    check_amount(&mut errors, "experience_years", i.experience_years);
    check_amount(&mut errors, "daily_cost", i.daily_cost);
    check_date(&mut errors, "availability_date", i.availability_date.as_deref());
    refs.check(&mut errors, "contract_type_id", RefList::ContractType, i.contract_type_id);
    check_user(conn, &mut errors, "owner_id", i.owner_id)?;
    check_required(
        &mut errors,
        &settings,
        "resource",
        &[
            ("title", i.title.is_some()),
            ("email", i.email.is_some()),
            ("phone", i.phone.is_some()),
            ("location", i.location.is_some()),
            ("skills", !i.skills.is_empty()),
            ("availability_date", i.availability_date.is_some()),
            ("contract_type_id", i.contract_type_id.is_some()),
        ],
    );
    errors.finish()?;
    if let Some(email) = &i.email
        && let Some((id, name)) = xpeperience_store::resources::find_by_email(conn, email, exclude)?
    {
        return Err(DomainError::conflict(
            format!("{name} utilise déjà cette adresse e-mail."),
            Some(serde_json::json!({ "entity": "resource", "id": id, "name": name })),
        ));
    }
    Ok(i)
}

fn trace_resource(conn: &Connection, me: &AuthUser, r: &Resource, action: &str, changes: Vec<xpeperience_domain::model::FieldChange>) -> Result<()> {
    trace(conn, me, Trace { entity: EntityKind::Resource, id: r.id, label: &r.full_name(), link: None }, action, &changes)
}

pub fn create_resource(conn: &mut Connection, me: &AuthUser, input: ResourceInput) -> Result<Resource> {
    me.require(Area::Resources, Action::Create)?;
    let tx = begin(conn)?;
    let refs = Refs::load(&tx)?;
    let mut i = clean_resource(&tx, &refs, input, None)?;
    if !can_edit_costs(me) {
        i.daily_cost = None;
    }
    i.owner_id = i.owner_id.or(Some(me.id));
    let kind = i.kind.unwrap_or(ResourceKind::Candidate);
    let id = xpeperience_store::resources::insert(&tx, &i, kind, me.id)?;
    let mut r = get(&tx, id)?;
    trace_resource(&tx, me, &r, "created", Vec::new())?;
    commit(tx)?;
    hide_resource_costs(me, &mut r);
    Ok(r)
}

pub fn update_resource(conn: &mut Connection, me: &AuthUser, id: Id, input: ResourceInput) -> Result<Resource> {
    me.require(Area::Resources, Action::Update)?;
    let tx = begin(conn)?;
    let old = get(&tx, id)?;
    let refs = Refs::load(&tx)?;
    let mut i = clean_resource(&tx, &refs, input, Some(id))?;
    if !can_edit_costs(me) {
        i.daily_cost = old.daily_cost;
    }
    let kind = i.kind.unwrap_or(old.kind);
    xpeperience_store::resources::update(&tx, id, &i, kind)?;
    let mut new = get(&tx, id)?;

    let kind_label = |k: ResourceKind| if k == ResourceKind::Candidate { "Candidat" } else { "Consultant" };
    let mut d = Diff::new();
    d.text("Statut", Some(kind_label(old.kind)), Some(kind_label(new.kind)));
    d.text("Prénom", Some(&old.first_name), Some(&new.first_name));
    d.text("Nom", Some(&old.last_name), Some(&new.last_name));
    d.text("Métier", old.title.as_deref(), new.title.as_deref());
    d.text("E-mail", old.email.as_deref(), new.email.as_deref());
    d.text("Téléphone", old.phone.as_deref(), new.phone.as_deref());
    d.text("Localisation", old.location.as_deref(), new.location.as_deref());
    d.text("LinkedIn", old.linkedin_url.as_deref(), new.linkedin_url.as_deref());
    d.tags("Compétences", &old.skills, &new.skills);
    d.num("Années d'expérience", old.experience_years, new.experience_years);
    d.text("Expérience", old.experience.as_deref(), new.experience.as_deref());
    d.text("Disponibilité", old.availability_date.as_deref(), new.availability_date.as_deref());
    d.text("Précision disponibilité", old.availability_note.as_deref(), new.availability_note.as_deref());
    d.reference("Type de contrat", &refs, old.contract_type_id, new.contract_type_id);
    if can_edit_costs(me) {
        d.num("CJM saisi", old.daily_cost, new.daily_cost);
    }
    d.text("Notes", old.notes.as_deref(), new.notes.as_deref());
    d.named("Responsable", old.owner.as_ref(), new.owner.as_ref());
    if !d.is_empty() {
        trace_resource(&tx, me, &new, "updated", d.into_vec())?;
    }
    commit(tx)?;
    hide_resource_costs(me, &mut new);
    Ok(new)
}

/// Un candidat recruté devient consultant ; son historique est conservé.
pub fn convert_to_consultant(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<Resource> {
    me.require(Area::Resources, Action::Update)?;
    let tx = begin(conn)?;
    let r = get(&tx, id)?;
    if r.kind == ResourceKind::Consultant {
        return Err(DomainError::rule("Cette ressource est déjà consultant."));
    }
    xpeperience_store::resources::set_kind(&tx, id, ResourceKind::Consultant)?;
    let change = xpeperience_domain::model::FieldChange { field: "Statut".into(), from: Some("Candidat".into()), to: Some("Consultant".into()) };
    trace_resource(&tx, me, &r, "updated", vec![change])?;
    let mut r = get(&tx, id)?;
    commit(tx)?;
    hide_resource_costs(me, &mut r);
    Ok(r)
}

pub fn archive_resource(conn: &mut Connection, me: &AuthUser, id: Id, archived: bool) -> Result<Resource> {
    me.require(Area::Resources, Action::Update)?;
    let tx = begin(conn)?;
    let r = get(&tx, id)?;
    xpeperience_store::resources::set_archived(&tx, id, archived)?;
    trace_resource(&tx, me, &r, if archived { "archived" } else { "restored" }, Vec::new())?;
    let mut r = get(&tx, id)?;
    commit(tx)?;
    hide_resource_costs(me, &mut r);
    Ok(r)
}

/// Supprime une ressource sans positionnement ni prestation. Renvoie ses documents à effacer du disque.
pub fn delete_resource(conn: &mut Connection, me: &AuthUser, id: Id) -> Result<Vec<ResourceFile>> {
    me.require(Area::Resources, Action::Delete)?;
    let tx = begin(conn)?;
    let r = get(&tx, id)?;
    let links = xpeperience_store::resources::links(&tx, id)?;
    if links > 0 {
        return Err(DomainError::rule(format!(
            "{} a {links} positionnement(s) ou prestation(s). Archivez cette fiche plutôt pour conserver l'historique.",
            r.full_name()
        )));
    }
    let files = xpeperience_store::resources::files(&tx, id)?;
    xpeperience_store::resources::delete(&tx, id)?;
    trace_resource(&tx, me, &r, "deleted", Vec::new())?;
    commit(tx)?;
    Ok(files)
}

// ───────────────────────────── Contrats RH ─────────────────────────────

pub fn contracts(conn: &Connection, me: &AuthUser, resource_id: Id) -> Result<Vec<HrContract>> {
    me.require(Area::Resources, Action::Read)?;
    me.require(Area::Costs, Action::Read)?;
    let mut list = xpeperience_store::resources::contracts(conn, resource_id)?;
    list.iter_mut().for_each(|c| hide_contract_costs(me, c));
    Ok(list)
}

fn clean_contract(refs: &Refs, i: HrContractInput) -> Result<HrContractInput> {
    let i = HrContractInput {
        start_date: i.start_date.trim().to_string(),
        end_date: clean(i.end_date),
        notes: clean(i.notes),
        ..i
    };
    let mut errors = Errors::new();
    if i.start_date.is_empty() {
        errors.add("start_date", "Indiquez la date de début du contrat.");
    }
    check_date(&mut errors, "start_date", Some(&i.start_date).filter(|s| !s.is_empty()).map(|s| s.as_str()));
    check_date(&mut errors, "end_date", i.end_date.as_deref());
    check_range(&mut errors, "end_date", Some(&i.start_date), i.end_date.as_deref());
    check_amount(&mut errors, "daily_cost", i.daily_cost);
    refs.check(&mut errors, "contract_type_id", RefList::ContractType, i.contract_type_id);
    errors.finish()?;
    Ok(i)
}

fn contract_change(c: &HrContractInput, before: Option<&HrContract>) -> xpeperience_domain::model::FieldChange {
    let fmt = |start: &str, end: Option<&str>| format!("{start} → {}", end.unwrap_or("sans fin"));
    xpeperience_domain::model::FieldChange {
        field: "Contrat RH".into(),
        from: before.map(|b| fmt(&b.start_date, b.end_date.as_deref())),
        to: Some(fmt(&c.start_date, c.end_date.as_deref())),
    }
}

pub fn create_contract(conn: &mut Connection, me: &AuthUser, resource_id: Id, input: HrContractInput) -> Result<HrContract> {
    me.require(Area::Resources, Action::Update)?;
    me.require(Area::Costs, Action::Update)?;
    let tx = begin(conn)?;
    let r = get(&tx, resource_id)?;
    let refs = Refs::load(&tx)?;
    let i = clean_contract(&refs, input)?;
    let id = xpeperience_store::resources::insert_contract(&tx, resource_id, &i)?;
    trace_resource(&tx, me, &r, "updated", vec![contract_change(&i, None)])?;
    commit(tx)?;
    xpeperience_store::resources::contract(conn, id)?.ok_or(DomainError::NotFound("Contrat"))
}

pub fn update_contract(conn: &mut Connection, me: &AuthUser, id: Id, input: HrContractInput) -> Result<HrContract> {
    me.require(Area::Resources, Action::Update)?;
    me.require(Area::Costs, Action::Update)?;
    let tx = begin(conn)?;
    let old = xpeperience_store::resources::contract(&tx, id)?.ok_or(DomainError::NotFound("Contrat"))?;
    let r = get(&tx, old.resource_id)?;
    let refs = Refs::load(&tx)?;
    let i = clean_contract(&refs, input)?;
    xpeperience_store::resources::update_contract(&tx, id, &i)?;
    trace_resource(&tx, me, &r, "updated", vec![contract_change(&i, Some(&old))])?;
    commit(tx)?;
    xpeperience_store::resources::contract(conn, id)?.ok_or(DomainError::NotFound("Contrat"))
}

pub fn delete_contract(conn: &Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Resources, Action::Update)?;
    me.require(Area::Costs, Action::Update)?;
    xpeperience_store::resources::contract(conn, id)?.ok_or(DomainError::NotFound("Contrat"))?;
    xpeperience_store::resources::delete_contract(conn, id)?;
    Ok(())
}

// ───────────────────────────── Documents ─────────────────────────────

pub const FILE_KINDS: &[&str] = &["cv", "skills_file", "other"];
pub const MAX_FILE_SIZE: usize = 15 * 1024 * 1024;
pub const ALLOWED_MIME: &[&str] = &[
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.oasis.opendocument.text",
    "text/plain",
    "image/png",
    "image/jpeg",
];

pub fn files(conn: &Connection, me: &AuthUser, resource_id: Id) -> Result<Vec<ResourceFile>> {
    me.require(Area::Resources, Action::Read)?;
    Ok(xpeperience_store::resources::files(conn, resource_id)?)
}

pub fn file(conn: &Connection, me: &AuthUser, id: Id) -> Result<ResourceFile> {
    me.require(Area::Resources, Action::Read)?;
    xpeperience_store::resources::file(conn, id)?.ok_or(DomainError::NotFound("Document"))
}

/// Vérifie un envoi de document avant son écriture sur le disque.
pub fn check_upload(conn: &Connection, me: &AuthUser, resource_id: Id, kind: &str, mime: &str, size: usize) -> Result<()> {
    me.require(Area::Resources, Action::Update)?;
    get(conn, resource_id)?;
    if !FILE_KINDS.contains(&kind) {
        return Err(DomainError::invalid("kind", "Type de document inconnu."));
    }
    if !ALLOWED_MIME.contains(&mime) {
        return Err(DomainError::invalid("file", "Format non accepté : PDF, Word, OpenDocument, texte ou image."));
    }
    if size == 0 || size > MAX_FILE_SIZE {
        return Err(DomainError::invalid("file", "Le fichier doit faire moins de 15 Mo."));
    }
    Ok(())
}

/// Nom de fichier affichable, sans chemin ni caractère de contrôle.
pub fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base.chars().filter(|c| !c.is_control()).collect::<String>().trim().to_string();
    let cleaned: String = cleaned.chars().take(160).collect();
    if cleaned.is_empty() { "document".into() } else { cleaned }
}

#[allow(clippy::too_many_arguments)]
pub fn add_file(
    conn: &Connection,
    me: &AuthUser,
    resource_id: Id,
    kind: &str,
    filename: &str,
    stored_name: &str,
    mime: &str,
    size: usize,
) -> Result<ResourceFile> {
    check_upload(conn, me, resource_id, kind, mime, size)?;
    let r = get(conn, resource_id)?;
    let id = xpeperience_store::resources::insert_file(
        conn,
        &xpeperience_store::resources::NewFile { resource_id, kind, filename, stored_name, mime, size: size as i64 },
        me.id,
    )?;
    let change = xpeperience_domain::model::FieldChange { field: "Document".into(), from: None, to: Some(filename.to_string()) };
    trace_resource(conn, me, &r, "updated", vec![change])?;
    xpeperience_store::resources::file(conn, id)?.ok_or(DomainError::NotFound("Document"))
}

/// Supprime l'enregistrement d'un document et renvoie ses informations pour effacer le fichier.
pub fn remove_file(conn: &Connection, me: &AuthUser, id: Id) -> Result<ResourceFile> {
    me.require(Area::Resources, Action::Update)?;
    let f = xpeperience_store::resources::file(conn, id)?.ok_or(DomainError::NotFound("Document"))?;
    let r = get(conn, f.resource_id)?;
    xpeperience_store::resources::delete_file(conn, id)?;
    let change = xpeperience_domain::model::FieldChange { field: "Document".into(), from: Some(f.filename.clone()), to: None };
    trace_resource(conn, me, &r, "updated", vec![change])?;
    Ok(f)
}
