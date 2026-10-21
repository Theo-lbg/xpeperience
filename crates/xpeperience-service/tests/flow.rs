//! Parcours métier complet sur une base en mémoire.
//! Toutes les données créées ici sont fictives et n'existent que pendant le test.

use xpeperience_domain::DomainError;
use xpeperience_domain::model::*;
use xpeperience_domain::permissions::{Action, Area};
use xpeperience_service::{AuthUser, admin, auth, crm, needs, positionings, projects, resources};
use xpeperience_store::Connection;

fn setup() -> (Connection, AuthUser) {
    let mut conn = xpeperience_store::open_in_memory().unwrap();
    let (admin, _token) = auth::setup(
        &mut conn,
        SetupInput {
            email: "admin.fictif@example.test".into(),
            first_name: "Admin".into(),
            last_name: "Fictif".into(),
            password: "motdepasse-test".into(),
        },
    )
    .unwrap();
    (conn, admin)
}

fn ref_id(conn: &Connection, list: &str, category: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM ref_items WHERE list = ?1 AND category = ?2 ORDER BY position LIMIT 1",
        [list, category],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn from_company_to_project_without_known_daily_cost() {
    let (mut conn, me) = setup();

    let company = crm::create_company(&mut conn, &me, CompanyInput { name: "Société Fictive".into(), ..Default::default() }).unwrap();
    // Même nom, accents et casse différents : refusé comme doublon.
    let dup = crm::create_company(&mut conn, &me, CompanyInput { name: "societe fictive".into(), ..Default::default() });
    assert!(matches!(dup, Err(DomainError::Conflict { .. })));

    let contact = crm::create_contact(
        &mut conn,
        &me,
        ContactInput { company_id: Some(company.id), first_name: Some("Camille".into()), last_name: "Test".into(), ..Default::default() },
    )
    .unwrap();
    let again = crm::create_contact(
        &mut conn,
        &me,
        ContactInput { company_id: Some(company.id), first_name: Some("Camille".into()), last_name: "TEST".into(), ..Default::default() },
    );
    assert!(matches!(again, Err(DomainError::Conflict { .. })), "doublon de contact détecté");

    // Un besoin minimal : titre et contact suffisent ; la société vient du contact.
    let need = needs::create_need(
        &mut conn,
        &me,
        NeedInput { title: "Développeur Rust".into(), contact_id: Some(contact.id), skills: vec!["Rust".into()], ..Default::default() },
    )
    .unwrap();
    assert_eq!(need.company.as_ref().map(|c| c.id), Some(company.id));

    // Ressource sans CJM : rien ne bloque.
    let resource = resources::create_resource(
        &mut conn,
        &me,
        ResourceInput { first_name: "Alex".into(), last_name: "Exemple".into(), skills: vec!["Rust".into(), "Vue.js".into()], ..Default::default() },
    )
    .unwrap();
    assert_eq!(resource.effective_daily_cost, None);

    let suggestions = positionings::suggestions(&conn, &me, need.id).unwrap();
    assert_eq!(suggestions.first().map(|s| s.resource.id), Some(resource.id));

    let saved = positionings::create_positioning(
        &mut conn,
        &me,
        PositioningInput { need_id: Some(need.id), resource_id: Some(resource.id), daily_rate: Some(600.0), days: Some(40.0), start_date: Some("2026-11-02".into()), end_date: Some("2027-01-29".into()), ..Default::default() },
    )
    .unwrap();
    assert_eq!(saved.positioning.daily_cost, None);

    // Passage à « gagné » : l'application propose de créer le projet.
    let won = ref_id(&conn, "positioning_state", "won");
    let saved = positionings::update_positioning(
        &mut conn,
        &me,
        saved.positioning.id,
        PositioningInput { state_id: Some(won), daily_rate: Some(600.0), days: Some(40.0), start_date: Some("2026-11-02".into()), end_date: Some("2027-01-29".into()), ..Default::default() },
    )
    .unwrap();
    assert!(saved.suggest_project);

    let project = positionings::convert_positioning(&mut conn, &me, saved.positioning.id, ConvertInput { mode: "new".into(), project_id: None }).unwrap();
    assert_eq!(project.company.as_ref().map(|c| c.id), Some(company.id));
    assert_eq!(project.start_date.as_deref(), Some("2026-11-02"));
    assert_eq!(project.end_date.as_deref(), Some("2027-01-29"));
    assert_eq!(project.resources.len(), 1);

    let need = needs::need(&conn, &me, need.id).unwrap();
    assert_eq!(need.project.as_ref().map(|p| p.id), Some(project.id));
    let need_won = ref_id(&conn, "need_state", "won");
    assert_eq!(need.state_id, need_won, "le besoin passe gagné");

    // Prolongations : signée → fin du projet ; prévisionnelle → fin prévisionnelle seulement.
    let delivery = projects::deliveries(&conn, &me, &DeliveryFilter { project_id: Some(project.id), resource_id: None }).unwrap().remove(0);
    assert_eq!(delivery.daily_cost, None, "le CJM reste vide");
    projects::create_extension(&mut conn, &me, delivery.id, ExtensionInput { status: Some(ExtensionStatus::Signed), start_date: Some("2027-01-30".into()), end_date: Some("2027-03-31".into()), ..Default::default() }).unwrap();
    projects::create_extension(&mut conn, &me, delivery.id, ExtensionInput { status: Some(ExtensionStatus::Forecast), start_date: Some("2027-04-01".into()), end_date: Some("2027-06-30".into()), ..Default::default() }).unwrap();
    let overlap = projects::create_extension(&mut conn, &me, delivery.id, ExtensionInput { start_date: Some("2027-03-01".into()), end_date: Some("2027-04-15".into()), ..Default::default() });
    assert!(matches!(overlap, Err(DomainError::Validation(_))));

    let project = projects::project(&conn, &me, project.id).unwrap();
    assert_eq!(project.end_date.as_deref(), Some("2027-03-31"));
    assert_eq!(project.forecast_end_date.as_deref(), Some("2027-06-30"));

    // Le CJM peut être complété plus tard.
    let updated = projects::update_delivery(
        &mut conn,
        &me,
        delivery.id,
        DeliveryInput { resource_id: Some(resource.id), start_date: Some("2026-11-02".into()), end_date: Some("2027-01-29".into()), daily_rate: Some(600.0), sold_days: Some(40.0), daily_cost: Some(420.0), ..Default::default() },
    )
    .unwrap();
    assert_eq!(updated.daily_cost, Some(420.0));

    let history = xpeperience_service::overview::history(&conn, &me, EntityKind::Project, project.id).unwrap();
    assert!(history.iter().any(|h| h.action == "created"));
}

#[test]
fn permissions_are_enforced_on_the_server() {
    let (mut conn, me) = setup();
    let roles = admin::roles(&conn, &me).unwrap();
    let readonly = roles.iter().find(|r| r.name == "Lecture seule").unwrap();
    let user = admin::create_user(
        &conn,
        &me,
        UserInput { email: "lecteur.fictif@example.test".into(), first_name: "Lecteur".into(), last_name: "Fictif".into(), role_id: readonly.id, active: None, password: Some("motdepasse-test".into()) },
    )
    .unwrap();
    let reader = auth::load_user(&conn, user.id).unwrap();

    assert!(reader.can(Area::Crm, Action::Read));
    let denied = crm::create_company(&mut conn, &reader, CompanyInput { name: "Refusée".into(), ..Default::default() });
    assert!(matches!(denied, Err(DomainError::Forbidden)));

    // Sans droit sur les coûts, le CJM n'est jamais renvoyé.
    let r = resources::create_resource(
        &mut conn,
        &me,
        ResourceInput { first_name: "Dominique".into(), last_name: "Exemple".into(), daily_cost: Some(390.0), ..Default::default() },
    )
    .unwrap();
    assert_eq!(r.effective_daily_cost, Some(390.0));
    let seen = resources::resource(&conn, &reader, r.id).unwrap();
    assert_eq!(seen.effective_daily_cost, None);
    assert!(resources::contracts(&conn, &reader, r.id).is_err());

    // Le dernier administrateur ne peut pas se retirer ses droits.
    let me_user = xpeperience_store::users::get(&conn, me.id).unwrap().unwrap();
    let demote = admin::update_user(
        &mut conn,
        &me,
        me.id,
        UserInput { email: me_user.email, first_name: me_user.first_name, last_name: me_user.last_name, role_id: readonly.id, active: Some(true), password: None },
    );
    assert!(matches!(demote, Err(DomainError::Rule(_))));
}

#[test]
fn login_session_and_linkedin_import() {
    let (mut conn, me) = setup();
    let (_, token) = auth::login(&conn, LoginInput { email: "ADMIN.fictif@example.test".into(), password: "motdepasse-test".into() }).unwrap();
    assert_eq!(auth::session_user(&conn, &token).unwrap().id, me.id);
    assert!(auth::login(&conn, LoginInput { email: "admin.fictif@example.test".into(), password: "faux".into() }).is_err());

    let created = admin::create_token(&conn, &me, None).unwrap();
    let ext_user = auth::token_user(&conn, &created.secret).unwrap();
    assert_eq!(ext_user.id, me.id);

    let profile = xpeperience_service::linkedin::Profile {
        first_name: Some("Sam".into()),
        last_name: Some("Profil".into()),
        job_title: Some("DSI".into()),
        company_name: Some("Entreprise Fictive".into()),
        location: Some("Lille".into()),
        linkedin_url: Some("https://www.linkedin.com/in/sam-profil-fictif/?trk=x".into()),
        ..Default::default()
    };
    let lookup = xpeperience_service::linkedin::lookup(&conn, &ext_user, &profile).unwrap();
    assert!(lookup.duplicates.is_empty());

    let imported = xpeperience_service::linkedin::import(
        &mut conn,
        &ext_user,
        xpeperience_service::linkedin::ImportRequest {
            profile: profile.clone(),
            company: xpeperience_service::linkedin::CompanyChoice::New { name: "Entreprise Fictive".into() },
            on_duplicate: None,
            duplicate_id: None,
        },
    )
    .unwrap();
    let contact = crm::contact(&conn, &me, imported.contact.id).unwrap();
    assert_eq!(contact.source, "linkedin");
    assert!(contact.imported_at.is_some());
    assert_eq!(contact.linkedin_url.as_deref(), Some("https://www.linkedin.com/in/sam-profil-fictif"));

    // Second import du même profil : doublon signalé, puis mise à jour sur demande.
    let lookup = xpeperience_service::linkedin::lookup(&conn, &ext_user, &profile).unwrap();
    assert_eq!(lookup.duplicates.len(), 1);
    assert_eq!(lookup.exact_company.as_ref().map(|c| c.id), contact.company.as_ref().map(|c| c.id));
    let updated = xpeperience_service::linkedin::import(
        &mut conn,
        &ext_user,
        xpeperience_service::linkedin::ImportRequest {
            profile: xpeperience_service::linkedin::Profile { job_title: Some("Directeur des systèmes d'information".into()), ..profile },
            company: xpeperience_service::linkedin::CompanyChoice::None,
            on_duplicate: Some("update".into()),
            duplicate_id: Some(contact.id),
        },
    )
    .unwrap();
    assert_eq!(updated.status, "updated");
    assert_eq!(crm::contact(&conn, &me, contact.id).unwrap().job_title.as_deref(), Some("Directeur des systèmes d'information"));

    admin::revoke_token(&conn, &me, created.token.id).unwrap();
    assert!(auth::token_user(&conn, &created.secret).is_err());
}
