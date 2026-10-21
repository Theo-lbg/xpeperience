//! Import d'un contact depuis l'extension LinkedIn.
//!
//! L'extension n'envoie que ce que l'utilisateur a vu et validé dans l'aperçu.
//! Chaque import passe par les mêmes droits et contrôles de doublons que la
//! saisie manuelle.

use xpeperience_domain::model::{CompanyInput, ContactInput, DuplicateContact, Named};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::text::{clean, clean_required, linkedin_key};
use xpeperience_domain::{DomainError, Id, Result};
use xpeperience_store::Connection;
use xpeperience_store::contacts::Origin;
use serde::{Deserialize, Serialize};

use crate::AuthUser;
use crate::crm::{apply_contact_update, clean_contact, find_duplicates, insert_company, insert_contact};
use crate::support::{begin, commit};

/// Informations lues sur le profil puis vérifiées par l'utilisateur.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Profile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub job_title: Option<String>,
    pub company_name: Option<String>,
    pub location: Option<String>,
    pub linkedin_url: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CompanyChoice {
    Existing { id: Id },
    New { name: String },
    None,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    pub profile: Profile,
    pub company: CompanyChoice,
    /// `create` (malgré les doublons) ou `update` (compléter le contact `duplicate_id`).
    pub on_duplicate: Option<String>,
    pub duplicate_id: Option<Id>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub app_name: String,
    pub user: String,
    pub enabled: bool,
    pub allow_company_creation: bool,
    pub can_import: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Lookup {
    pub duplicates: Vec<DuplicateContact>,
    /// Société au nom identique, proposée par défaut.
    pub exact_company: Option<Named>,
    pub companies: Vec<Named>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Imported {
    /// `created` ou `updated`.
    pub status: String,
    pub contact: Named,
    pub company: Option<Named>,
}

fn ensure_enabled(conn: &Connection) -> Result<xpeperience_domain::settings::Settings> {
    let settings = xpeperience_store::settings::load(conn)?;
    if !settings.extension.enabled {
        return Err(DomainError::rule("L'import LinkedIn est désactivé dans les réglages de XpePerience."));
    }
    Ok(settings)
}

pub fn status(conn: &Connection, me: &AuthUser) -> Result<Status> {
    let settings = xpeperience_store::settings::load(conn)?;
    Ok(Status {
        app_name: settings.app_name,
        user: me.name(),
        enabled: settings.extension.enabled,
        allow_company_creation: settings.extension.allow_company_creation && me.can(Area::Crm, Action::Create),
        can_import: me.can(Area::Crm, Action::Create),
    })
}

fn to_contact(p: &Profile, company_id: Option<Id>) -> ContactInput {
    ContactInput {
        company_id,
        first_name: clean(p.first_name.clone()),
        last_name: p.last_name.clone().unwrap_or_default(),
        job_title: clean(p.job_title.clone()),
        email: clean(p.email.clone()),
        phone: clean(p.phone.clone()),
        mobile: None,
        linkedin_url: clean(p.linkedin_url.clone()).and_then(|u| linkedin_key(&u).map(|k| format!("https://www.{k}"))),
        location: clean(p.location.clone()),
        notes: None,
        owner_id: None,
        force: false,
    }
}

/// Doublons possibles et sociétés correspondantes, affichés dans l'aperçu.
pub fn lookup(conn: &Connection, me: &AuthUser, profile: &Profile) -> Result<Lookup> {
    me.require(Area::Crm, Action::Read)?;
    ensure_enabled(conn)?;
    let input = to_contact(profile, None);
    let duplicates =
        if clean_required(&input.last_name).is_empty() { Vec::new() } else { find_duplicates(conn, &input, None)? };
    let (exact_company, companies) = match clean(profile.company_name.clone()) {
        Some(name) => (xpeperience_store::companies::find_by_name(conn, &name, None)?, xpeperience_store::companies::similar(conn, &name, 6)?),
        None => (None, Vec::new()),
    };
    Ok(Lookup { duplicates, exact_company, companies })
}

/// Recherche de sociétés pour le sélecteur de l'aperçu.
pub fn companies(conn: &Connection, me: &AuthUser, q: &str) -> Result<Vec<Named>> {
    me.require(Area::Crm, Action::Read)?;
    ensure_enabled(conn)?;
    Ok(xpeperience_store::companies::similar(conn, q, 10)?)
}

pub fn import(conn: &mut Connection, me: &AuthUser, req: ImportRequest) -> Result<Imported> {
    me.require(Area::Crm, Action::Create)?;
    let tx = begin(conn)?;
    let settings = ensure_enabled(&tx)?;

    let company = match &req.company {
        CompanyChoice::None => None,
        CompanyChoice::Existing { id } => {
            let c = xpeperience_store::companies::get(&tx, *id)?.ok_or_else(|| DomainError::invalid("company", "Société introuvable."))?;
            Some(Named { id: c.id, name: c.name })
        }
        CompanyChoice::New { name } => {
            let name = clean_required(name);
            match xpeperience_store::companies::find_by_name(&tx, &name, None)? {
                // Une société du même nom existe : on l'utilise plutôt que de créer un doublon.
                Some(existing) => Some(existing),
                None => {
                    if !settings.extension.allow_company_creation {
                        return Err(DomainError::rule("La création de société depuis l'extension est désactivée."));
                    }
                    let c = insert_company(&tx, me, CompanyInput { name, ..Default::default() })?;
                    Some(Named { id: c.id, name: c.name })
                }
            }
        }
    };

    let mut input = to_contact(&req.profile, company.as_ref().map(|c| c.id));
    let now = xpeperience_store::now();
    let (status, contact) = match (req.on_duplicate.as_deref(), req.duplicate_id) {
        (Some("update"), Some(existing_id)) => {
            me.require(Area::Crm, Action::Update)?;
            let current = xpeperience_store::contacts::get(&tx, existing_id)?.ok_or(DomainError::NotFound("Contact"))?;
            // Complète la fiche existante sans effacer ce qui y figure déjà.
            let merged = ContactInput {
                company_id: input.company_id.or(current.company.map(|c| c.id)),
                first_name: input.first_name.or(current.first_name),
                last_name: if input.last_name.trim().is_empty() { current.last_name } else { input.last_name },
                job_title: input.job_title.or(current.job_title),
                email: current.email.or(input.email),
                phone: current.phone.or(input.phone),
                mobile: current.mobile,
                linkedin_url: input.linkedin_url.or(current.linkedin_url),
                location: input.location.or(current.location),
                notes: current.notes,
                owner_id: current.owner.map(|o| o.id),
                force: true,
            };
            clean_contact(&tx, merged.clone())?;
            let c = apply_contact_update(&tx, me, existing_id, merged, "imported")?;
            xpeperience_store::contacts::mark_imported(&tx, existing_id, &now)?;
            ("updated", c)
        }
        (Some("create"), _) => {
            input.force = true;
            ("created", insert_contact(&tx, me, input, Origin { source: "linkedin", imported_at: Some(&now) })?)
        }
        _ => ("created", insert_contact(&tx, me, input, Origin { source: "linkedin", imported_at: Some(&now) })?),
    };

    if settings.extension.add_import_note {
        xpeperience_store::activities::insert(
            &tx,
            &xpeperience_store::activities::NewActivity {
                kind: xpeperience_domain::model::ActivityKind::Note,
                occurred_at: &now,
                comment: if status == "created" {
                    "Contact importé depuis LinkedIn."
                } else {
                    "Fiche complétée depuis LinkedIn."
                },
                due_date: None,
                company_id: contact.company.as_ref().map(|c| c.id),
                contact_id: Some(contact.id),
                need_id: None,
                resource_id: None,
                positioning_id: None,
                project_id: None,
                owner_id: Some(me.id),
            },
            me.id,
        )?;
    }
    commit(tx)?;
    Ok(Imported { status: status.into(), contact: Named { id: contact.id, name: contact.full_name() }, company })
}
