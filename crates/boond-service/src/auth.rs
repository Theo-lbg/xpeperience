//! Comptes, connexion, sessions et jetons d'appairage.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng};
use argon2::Argon2;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use boond_domain::error::Errors;
use boond_domain::model::{LoginInput, Me, PasswordChange, Preferences, SetupInput};
use boond_domain::permissions::{Action, Area, Permissions};
use boond_domain::rules::check_email;
use boond_domain::text::clean_required;
use boond_domain::{DomainError, Id, Result};
use boond_store::Connection;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Utilisateur authentifié et ses droits, chargés à chaque requête.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Id,
    pub first_name: String,
    pub last_name: String,
    pub is_admin: bool,
    pub permissions: Permissions,
}

impl AuthUser {
    pub fn can(&self, area: Area, action: Action) -> bool {
        self.is_admin || self.permissions.can(area, action)
    }

    pub fn require(&self, area: Area, action: Action) -> Result<()> {
        if self.can(area, action) { Ok(()) } else { Err(DomainError::Forbidden) }
    }

    pub fn name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// Durée d'une session sans activité : l'application rouvre directement le tableau de bord.
const SESSION_DAYS: i64 = 60;
pub const MIN_PASSWORD_LEN: usize = 8;

fn rfc3339(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub(crate) fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| DomainError::Internal(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

pub(crate) fn check_password(errors: &mut Errors, field: &str, password: &str) {
    if password.chars().count() < MIN_PASSWORD_LEN {
        errors.add(field, format!("Au moins {MIN_PASSWORD_LEN} caractères."));
    }
}

/// Empreinte SHA-256 d'un secret (jeton de session ou d'appairage).
pub fn hash_secret(secret: &str) -> String {
    Sha256::digest(secret.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Nouveau secret aléatoire (256 bits) avec un préfixe lisible.
pub(crate) fn new_secret(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!("{prefix}{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub fn load_user(conn: &Connection, id: Id) -> Result<AuthUser> {
    let user = boond_store::users::get(conn, id)?.ok_or(DomainError::Unauthorized)?;
    if !user.active {
        return Err(DomainError::Unauthorized);
    }
    let role = boond_store::users::role(conn, user.role_id)?.ok_or(DomainError::Unauthorized)?;
    Ok(AuthUser {
        id: user.id,
        first_name: user.first_name,
        last_name: user.last_name,
        is_admin: role.is_admin,
        permissions: if role.is_admin { Permissions::all() } else { role.permissions },
    })
}

fn open_session(conn: &Connection, user_id: Id) -> Result<String> {
    let token = new_secret("");
    let expires = rfc3339(chrono::Utc::now() + chrono::Duration::days(SESSION_DAYS));
    boond_store::users::create_session(conn, &hash_secret(&token), user_id, &expires)?;
    boond_store::users::touch_login(conn, user_id)?;
    boond_store::users::purge_sessions(conn, &boond_store::now())?;
    Ok(token)
}

pub fn setup_required(conn: &Connection) -> Result<bool> {
    Ok(boond_store::users::count(conn)? == 0)
}

/// Premier lancement : crée le compte administrateur et ouvre sa session.
pub fn setup(conn: &mut Connection, input: SetupInput) -> Result<(AuthUser, String)> {
    let tx = conn.transaction().map_err(boond_store::StoreError::from)?;
    if boond_store::users::count(&tx)? > 0 {
        return Err(DomainError::rule("L'application est déjà configurée : connectez-vous."));
    }
    let email = input.email.trim().to_lowercase();
    let first = clean_required(&input.first_name);
    let last = clean_required(&input.last_name);
    let mut errors = Errors::new();
    if first.is_empty() {
        errors.add("first_name", "Indiquez votre prénom.");
    }
    if last.is_empty() {
        errors.add("last_name", "Indiquez votre nom.");
    }
    if email.is_empty() {
        errors.add("email", "Indiquez votre adresse e-mail.");
    }
    check_email(&mut errors, "email", Some(&email).filter(|e| !e.is_empty()).map(|e| e.as_str()));
    check_password(&mut errors, "password", &input.password);
    errors.finish()?;

    let role_id = boond_store::users::admin_role_id(&tx)?;
    let id = boond_store::users::insert(
        &tx,
        &boond_store::users::NewUser {
            email: &email,
            first_name: &first,
            last_name: &last,
            password_hash: &hash_password(&input.password)?,
            role_id,
            active: true,
        },
    )?;
    let token = open_session(&tx, id)?;
    let user = load_user(&tx, id)?;
    tx.commit().map_err(boond_store::StoreError::from)?;
    Ok((user, token))
}

pub fn login(conn: &Connection, input: LoginInput) -> Result<(AuthUser, String)> {
    let invalid = || DomainError::invalid("password", "E-mail ou mot de passe incorrect.");
    let email = input.email.trim().to_lowercase();
    let Some((user, hash)) = boond_store::users::find_for_login(conn, &email)? else {
        return Err(invalid());
    };
    if !verify_password(&input.password, &hash) {
        return Err(invalid());
    }
    if !user.active {
        return Err(DomainError::invalid("email", "Ce compte est désactivé. Contactez un administrateur."));
    }
    let token = open_session(conn, user.id)?;
    Ok((load_user(conn, user.id)?, token))
}

pub fn logout(conn: &Connection, token: &str) -> Result<()> {
    boond_store::users::delete_session(conn, &hash_secret(token))?;
    Ok(())
}

/// Utilisateur d'une session valide ; prolonge la session si elle a plus d'un jour.
pub fn session_user(conn: &Connection, token: &str) -> Result<AuthUser> {
    let hash = hash_secret(token);
    let now = chrono::Utc::now();
    let (user_id, expires_at) =
        boond_store::users::session_user(conn, &hash, &rfc3339(now))?.ok_or(DomainError::Unauthorized)?;
    let renew_after = rfc3339(now + chrono::Duration::days(SESSION_DAYS - 1));
    if expires_at < renew_after {
        boond_store::users::extend_session(conn, &hash, &rfc3339(now + chrono::Duration::days(SESSION_DAYS)))?;
    }
    load_user(conn, user_id)
}

/// Utilisateur associé à un jeton d'appairage de l'extension.
pub fn token_user(conn: &Connection, token: &str) -> Result<AuthUser> {
    let user_id = boond_store::users::token_user(conn, &hash_secret(token))?.ok_or(DomainError::Unauthorized)?;
    load_user(conn, user_id)
}

pub fn me(conn: &Connection, me: &AuthUser) -> Result<Me> {
    let user = boond_store::users::get(conn, me.id)?.ok_or(DomainError::Unauthorized)?;
    Ok(Me { user, permissions: me.permissions.clone(), preferences: boond_store::users::preferences(conn, me.id)? })
}

pub fn set_preferences(conn: &Connection, me: &AuthUser, mut prefs: Preferences) -> Result<Preferences> {
    if !["comfortable", "compact"].contains(&prefs.density.as_str()) {
        prefs.density = "comfortable".into();
    }
    if !["list", "kanban"].contains(&prefs.needs_view.as_str()) {
        prefs.needs_view = "list".into();
    }
    boond_store::users::set_preferences(conn, me.id, &prefs)?;
    Ok(prefs)
}

pub fn change_password(conn: &Connection, me: &AuthUser, input: PasswordChange, current_token: Option<&str>) -> Result<()> {
    let hash = boond_store::users::password_hash(conn, me.id)?.ok_or(DomainError::Unauthorized)?;
    let mut errors = Errors::new();
    if !input.current_password.as_deref().is_some_and(|p| verify_password(p, &hash)) {
        errors.add("current_password", "Mot de passe actuel incorrect.");
    }
    check_password(&mut errors, "new_password", &input.new_password);
    errors.finish()?;
    boond_store::users::set_password(conn, me.id, &hash_password(&input.new_password)?)?;
    // Les autres sessions de l'utilisateur sont fermées ; la session courante est recréée par l'appelant.
    boond_store::users::delete_user_sessions(conn, me.id)?;
    if let Some(t) = current_token {
        let expires = rfc3339(chrono::Utc::now() + chrono::Duration::days(SESSION_DAYS));
        boond_store::users::create_session(conn, &hash_secret(t), me.id, &expires)?;
    }
    Ok(())
}
