//! Réglages, listes paramétrables, utilisateurs, rôles et jetons d'appairage.

use xpeperience_domain::error::Errors;
use xpeperience_domain::model::{ApiToken, CreatedToken, Named, Role, RoleInput, User, UserInput};
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_domain::refs::{COLORS, RefItem, RefItemInput, RefList};
use xpeperience_domain::rules::check_email;
use xpeperience_domain::settings::Settings;
use xpeperience_domain::text::{clean, clean_required};
use xpeperience_domain::{DomainError, Id, Result};
use xpeperience_store::Connection;

use crate::AuthUser;
use crate::auth::{check_password, hash_password, hash_secret, new_secret};
use crate::support::{begin, commit};

// ───────────────────────────── Réglages ─────────────────────────────

/// Les réglages d'affichage sont lisibles par tout utilisateur connecté.
pub fn settings(conn: &Connection) -> Result<Settings> {
    Ok(xpeperience_store::settings::load(conn)?)
}

pub fn update_settings(conn: &Connection, me: &AuthUser, input: Settings) -> Result<Settings> {
    me.require(Area::Settings, Action::Update)?;
    let settings = input.validated()?;
    xpeperience_store::settings::save(conn, &settings)?;
    Ok(settings)
}

// ─────────────────────── Listes paramétrables ───────────────────────

pub fn refs(conn: &Connection) -> Result<Vec<RefItem>> {
    Ok(xpeperience_store::refs::all(conn)?)
}

fn clean_ref(list: RefList, input: RefItemInput) -> Result<(String, Option<String>, Option<String>, bool)> {
    let label = clean_required(&input.label);
    let category = clean(input.category);
    let color = clean(input.color);
    let mut errors = Errors::new();
    if label.is_empty() {
        errors.add("label", "Indiquez un libellé.");
    }
    match (&category, list.has_categories()) {
        (None, true) => errors.add("category", "Choisissez une catégorie."),
        (Some(c), true) if !list.is_valid_category(c) => errors.add("category", "Catégorie inconnue."),
        (Some(_), false) => errors.add("category", "Cette liste n'utilise pas de catégorie."),
        _ => {}
    }
    if color.as_deref().is_some_and(|c| !COLORS.contains(&c)) {
        errors.add("color", "Couleur inconnue.");
    }
    errors.finish()?;
    Ok((label, category, color, input.active.unwrap_or(true)))
}

/// Les états « en cours » servent d'état initial : il doit toujours en rester un actif.
fn keep_open_state(conn: &Connection, item: &RefItem, still_open_active: bool) -> Result<()> {
    let guarded = matches!(item.list, RefList::NeedState | RefList::PositioningState);
    if guarded
        && item.active
        && item.category.as_deref() == Some("open")
        && !still_open_active
        && xpeperience_store::refs::count_active_in_category(conn, item.list, "open")? <= 1
    {
        return Err(DomainError::rule("Gardez au moins un état actif de catégorie « En cours » : il sert d'état initial."));
    }
    Ok(())
}

pub fn create_ref(conn: &Connection, me: &AuthUser, list: RefList, input: RefItemInput) -> Result<RefItem> {
    me.require(Area::Settings, Action::Update)?;
    let (label, category, color, _) = clean_ref(list, input)?;
    let id = xpeperience_store::refs::insert(conn, list, &label, category.as_deref(), color.as_deref())?;
    xpeperience_store::refs::get(conn, id)?.ok_or(DomainError::NotFound("Élément"))
}

pub fn update_ref(conn: &Connection, me: &AuthUser, id: Id, input: RefItemInput) -> Result<RefItem> {
    me.require(Area::Settings, Action::Update)?;
    let item = xpeperience_store::refs::get(conn, id)?.ok_or(DomainError::NotFound("Élément"))?;
    let (label, category, color, active) = clean_ref(item.list, input)?;
    keep_open_state(conn, &item, active && category.as_deref() == Some("open"))?;
    xpeperience_store::refs::update(conn, id, &label, category.as_deref(), color.as_deref(), active)?;
    xpeperience_store::refs::get(conn, id)?.ok_or(DomainError::NotFound("Élément"))
}

pub fn delete_ref(conn: &Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Settings, Action::Update)?;
    let item = xpeperience_store::refs::get(conn, id)?.ok_or(DomainError::NotFound("Élément"))?;
    let used = xpeperience_store::refs::usage(conn, id)?;
    if used > 0 {
        return Err(DomainError::rule(format!(
            "« {} » est utilisé par {used} fiche(s) : désactivez-le plutôt que de le supprimer.",
            item.label
        )));
    }
    keep_open_state(conn, &item, false)?;
    xpeperience_store::refs::delete(conn, id)?;
    Ok(())
}

pub fn reorder_refs(conn: &mut Connection, me: &AuthUser, list: RefList, ids: Vec<Id>) -> Result<Vec<RefItem>> {
    me.require(Area::Settings, Action::Update)?;
    let tx = begin(conn)?;
    xpeperience_store::refs::reorder(&tx, list, &ids)?;
    commit(tx)?;
    refs(conn)
}

// ─────────────────────────── Utilisateurs ───────────────────────────

/// Annuaire des utilisateurs actifs, pour choisir un responsable.
pub fn directory(conn: &Connection) -> Result<Vec<Named>> {
    Ok(xpeperience_store::users::directory(conn)?)
}

pub fn users(conn: &Connection, me: &AuthUser) -> Result<Vec<User>> {
    me.require(Area::Users, Action::Read)?;
    Ok(xpeperience_store::users::list(conn)?)
}

struct CleanUser {
    email: String,
    first: String,
    last: String,
    role_id: Id,
    active: bool,
}

fn clean_user(conn: &Connection, input: &UserInput) -> Result<CleanUser> {
    let mut errors = Errors::new();
    let email = input.email.trim().to_lowercase();
    let first = clean_required(&input.first_name);
    let last = clean_required(&input.last_name);
    if first.is_empty() {
        errors.add("first_name", "Indiquez le prénom.");
    }
    if last.is_empty() {
        errors.add("last_name", "Indiquez le nom.");
    }
    if email.is_empty() {
        errors.add("email", "Indiquez l'adresse e-mail.");
    } else {
        check_email(&mut errors, "email", Some(&email));
    }
    if xpeperience_store::users::role(conn, input.role_id)?.is_none() {
        errors.add("role_id", "Rôle inconnu.");
    }
    errors.finish()?;
    Ok(CleanUser { email, first, last, role_id: input.role_id, active: input.active.unwrap_or(true) })
}

pub fn create_user(conn: &Connection, me: &AuthUser, input: UserInput) -> Result<User> {
    me.require(Area::Users, Action::Create)?;
    let u = clean_user(conn, &input)?;
    let password = input.password.unwrap_or_default();
    let mut errors = Errors::new();
    check_password(&mut errors, "password", &password);
    errors.finish()?;
    let id = xpeperience_store::users::insert(
        conn,
        &xpeperience_store::users::NewUser {
            email: &u.email,
            first_name: &u.first,
            last_name: &u.last,
            password_hash: &hash_password(&password)?,
            role_id: u.role_id,
            active: u.active,
        },
    )
    .map_err(|e| match DomainError::from(e) {
        DomainError::Conflict { .. } => DomainError::invalid("email", "Un utilisateur utilise déjà cette adresse."),
        other => other,
    })?;
    xpeperience_store::users::get(conn, id)?.ok_or(DomainError::NotFound("Utilisateur"))
}

pub fn update_user(conn: &mut Connection, me: &AuthUser, id: Id, input: UserInput) -> Result<User> {
    me.require(Area::Users, Action::Update)?;
    let u = clean_user(conn, &input)?;
    let tx = begin(conn)?;
    xpeperience_store::users::get(&tx, id)?.ok_or(DomainError::NotFound("Utilisateur"))?;
    if id == me.id && !u.active {
        return Err(DomainError::rule("Vous ne pouvez pas désactiver votre propre compte."));
    }
    xpeperience_store::users::update(&tx, id, &u.email, &u.first, &u.last, u.role_id, u.active).map_err(|e| {
        match DomainError::from(e) {
            DomainError::Conflict { .. } => DomainError::invalid("email", "Un utilisateur utilise déjà cette adresse."),
            other => other,
        }
    })?;
    if xpeperience_store::users::active_admins(&tx)? == 0 {
        return Err(DomainError::rule("Il doit rester au moins un administrateur actif."));
    }
    commit(tx)?;
    xpeperience_store::users::get(conn, id)?.ok_or(DomainError::NotFound("Utilisateur"))
}

pub fn reset_password(conn: &Connection, me: &AuthUser, id: Id, password: &str) -> Result<()> {
    me.require(Area::Users, Action::Update)?;
    xpeperience_store::users::get(conn, id)?.ok_or(DomainError::NotFound("Utilisateur"))?;
    let mut errors = Errors::new();
    check_password(&mut errors, "password", password);
    errors.finish()?;
    xpeperience_store::users::set_password(conn, id, &hash_password(password)?)?;
    xpeperience_store::users::delete_user_sessions(conn, id)?;
    Ok(())
}

// ─────────────────────────────── Rôles ───────────────────────────────

pub fn roles(conn: &Connection, me: &AuthUser) -> Result<Vec<Role>> {
    me.require(Area::Users, Action::Read)?;
    Ok(xpeperience_store::users::roles(conn)?)
}

fn clean_role(input: RoleInput) -> Result<RoleInput> {
    let name = clean_required(&input.name);
    if name.is_empty() {
        return Err(DomainError::invalid("name", "Indiquez un nom de rôle."));
    }
    Ok(RoleInput { name, description: clean(input.description), permissions: input.permissions.normalized() })
}

fn role_conflict(e: xpeperience_store::StoreError) -> DomainError {
    match DomainError::from(e) {
        DomainError::Conflict { .. } => DomainError::invalid("name", "Un rôle porte déjà ce nom."),
        other => other,
    }
}

pub fn create_role(conn: &Connection, me: &AuthUser, input: RoleInput) -> Result<Role> {
    me.require(Area::Users, Action::Create)?;
    let r = clean_role(input)?;
    let id = xpeperience_store::users::insert_role(conn, &r.name, r.description.as_deref(), &r.permissions).map_err(role_conflict)?;
    xpeperience_store::users::role(conn, id)?.ok_or(DomainError::NotFound("Rôle"))
}

pub fn update_role(conn: &Connection, me: &AuthUser, id: Id, input: RoleInput) -> Result<Role> {
    me.require(Area::Users, Action::Update)?;
    let existing = xpeperience_store::users::role(conn, id)?.ok_or(DomainError::NotFound("Rôle"))?;
    let r = clean_role(input)?;
    // Le rôle administrateur garde tous ses droits : seuls son nom et sa description changent.
    let perms = if existing.is_admin { existing.permissions } else { r.permissions };
    xpeperience_store::users::update_role(conn, id, &r.name, r.description.as_deref(), &perms).map_err(role_conflict)?;
    xpeperience_store::users::role(conn, id)?.ok_or(DomainError::NotFound("Rôle"))
}

pub fn delete_role(conn: &Connection, me: &AuthUser, id: Id) -> Result<()> {
    me.require(Area::Users, Action::Delete)?;
    let role = xpeperience_store::users::role(conn, id)?.ok_or(DomainError::NotFound("Rôle"))?;
    if role.is_admin {
        return Err(DomainError::rule("Le rôle administrateur ne peut pas être supprimé."));
    }
    if role.users_count > 0 {
        return Err(DomainError::rule(format!(
            "{} utilisateur(s) ont ce rôle : attribuez-leur un autre rôle avant de le supprimer.",
            role.users_count
        )));
    }
    xpeperience_store::users::delete_role(conn, id)?;
    Ok(())
}

// ───────────────────── Jetons d'appairage de l'extension ─────────────────────

/// Jetons de l'utilisateur ; un administrateur des utilisateurs voit tous les jetons.
pub fn tokens(conn: &Connection, me: &AuthUser) -> Result<Vec<ApiToken>> {
    let scope = if me.can(Area::Users, Action::Read) { None } else { Some(me.id) };
    Ok(xpeperience_store::users::tokens(conn, scope)?)
}

pub fn create_token(conn: &Connection, me: &AuthUser, name: Option<String>) -> Result<CreatedToken> {
    let name = clean(name).unwrap_or_else(|| "Extension LinkedIn".into());
    let secret = new_secret("xpe_");
    let prefix: String = secret.chars().take(10).collect();
    let id = xpeperience_store::users::insert_token(conn, me.id, &name, &prefix, &hash_secret(&secret))?;
    let token = xpeperience_store::users::token(conn, id)?.ok_or(DomainError::NotFound("Jeton"))?;
    Ok(CreatedToken { token, secret })
}

pub fn revoke_token(conn: &Connection, me: &AuthUser, id: Id) -> Result<()> {
    let token = xpeperience_store::users::token(conn, id)?.ok_or(DomainError::NotFound("Jeton"))?;
    if token.user_id != me.id {
        me.require(Area::Users, Action::Update)?;
    }
    xpeperience_store::users::revoke_token(conn, id)?;
    Ok(())
}
