//! Objets métier et formulaires échangés avec l'interface.
//!
//! Les dates sont des chaînes ISO (`AAAA-MM-JJ`), les horodatages des chaînes
//! RFC 3339 en UTC. Les montants sont en euros ; ils sont tous facultatifs.

use serde::{Deserialize, Serialize};

use crate::Id;
use crate::permissions::Permissions;

/// Référence courte vers un objet lié, pour l'affichage et la navigation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Named {
    pub id: Id,
    pub name: String,
}

// ───────────────────────── Utilisateurs et rôles ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Id,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role_id: Id,
    pub role_name: String,
    pub is_admin: bool,
    pub active: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// `comfortable` ou `compact`.
    pub density: String,
    /// Vue par défaut des besoins : `list` ou `kanban`.
    pub needs_view: String,
    pub sidebar_collapsed: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { density: "comfortable".into(), needs_view: "list".into(), sidebar_collapsed: false }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Me {
    pub user: User,
    pub permissions: Permissions,
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInput {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role_id: Id,
    pub active: Option<bool>,
    /// Mot de passe initial (création uniquement).
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetupInput {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PasswordChange {
    pub current_password: Option<String>,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Role {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub is_admin: bool,
    pub permissions: Permissions,
    pub users_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleInput {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Permissions,
}

/// Jeton d'appairage de l'extension (la valeur n'est montrée qu'une fois).
#[derive(Debug, Clone, Serialize)]
pub struct ApiToken {
    pub id: Id,
    pub name: String,
    pub prefix: String,
    pub user_id: Id,
    pub user_name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedToken {
    pub token: ApiToken,
    pub secret: String,
}

// ─────────────────────────────── CRM ───────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Company {
    pub id: Id,
    pub name: String,
    pub type_id: Option<Id>,
    pub sector_id: Option<Id>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
    pub owner: Option<Named>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub contacts_count: i64,
    pub open_needs_count: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CompanyInput {
    pub name: String,
    pub type_id: Option<Id>,
    pub sector_id: Option<Id>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
    pub owner_id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CompanyFilter {
    pub q: Option<String>,
    pub type_id: Option<Id>,
    pub sector_id: Option<Id>,
    pub owner_id: Option<Id>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Contact {
    pub id: Id,
    pub company: Option<Named>,
    pub first_name: Option<String>,
    pub last_name: String,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub linkedin_url: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub owner: Option<Named>,
    /// `manual` ou `linkedin`.
    pub source: String,
    pub imported_at: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Contact {
    pub fn full_name(&self) -> String {
        person_name(self.first_name.as_deref(), &self.last_name)
    }
}

pub fn person_name(first: Option<&str>, last: &str) -> String {
    match first {
        Some(f) if !f.is_empty() => format!("{f} {last}"),
        _ => last.to_string(),
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContactInput {
    pub company_id: Option<Id>,
    pub first_name: Option<String>,
    pub last_name: String,
    pub job_title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub linkedin_url: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub owner_id: Option<Id>,
    /// Confirme la création malgré des doublons possibles.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContactFilter {
    pub q: Option<String>,
    pub company_id: Option<Id>,
    pub owner_id: Option<Id>,
    pub type_id: Option<Id>,
    pub sector_id: Option<Id>,
    pub source: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateContact {
    pub id: Id,
    pub name: String,
    pub company: Option<String>,
    pub job_title: Option<String>,
    /// Ce qui a déclenché la correspondance.
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Call,
    Meeting,
    Email,
    FollowUp,
    Note,
}

impl ActivityKind {
    pub fn code(self) -> &'static str {
        match self {
            ActivityKind::Call => "call",
            ActivityKind::Meeting => "meeting",
            ActivityKind::Email => "email",
            ActivityKind::FollowUp => "follow_up",
            ActivityKind::Note => "note",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        [Self::Call, Self::Meeting, Self::Email, Self::FollowUp, Self::Note].into_iter().find(|k| k.code() == code)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub id: Id,
    pub kind: ActivityKind,
    pub occurred_at: String,
    pub comment: String,
    pub due_date: Option<String>,
    pub done_at: Option<String>,
    pub company: Option<Named>,
    pub contact: Option<Named>,
    pub need: Option<Named>,
    pub resource: Option<Named>,
    pub positioning_id: Option<Id>,
    pub project: Option<Named>,
    pub owner: Option<Named>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ActivityInput {
    pub kind: Option<ActivityKind>,
    pub occurred_at: Option<String>,
    #[serde(default)]
    pub comment: String,
    pub due_date: Option<String>,
    pub company_id: Option<Id>,
    pub contact_id: Option<Id>,
    pub need_id: Option<Id>,
    pub resource_id: Option<Id>,
    pub positioning_id: Option<Id>,
    pub project_id: Option<Id>,
    pub owner_id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ActivityFilter {
    pub company_id: Option<Id>,
    pub contact_id: Option<Id>,
    pub need_id: Option<Id>,
    pub resource_id: Option<Id>,
    pub positioning_id: Option<Id>,
    pub project_id: Option<Id>,
    pub owner_id: Option<Id>,
    /// Uniquement les rappels (avec échéance) non terminés.
    pub reminders: Option<bool>,
    /// Échéance au plus tard à cette date.
    pub due_before: Option<String>,
    pub limit: Option<i64>,
}

// ───────────────────────────── Besoins ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Need {
    pub id: Id,
    pub title: String,
    pub company: Option<Named>,
    pub contact: Option<Named>,
    pub description: Option<String>,
    pub skills: Vec<String>,
    pub type_id: Option<Id>,
    pub state_id: Id,
    pub origin_id: Option<Id>,
    pub budget: Option<f64>,
    pub duration_months: Option<f64>,
    pub planned_days: Option<f64>,
    pub start_date: Option<String>,
    pub location: Option<String>,
    pub owner: Option<Named>,
    pub project: Option<Named>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub positionings_count: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NeedInput {
    pub title: String,
    pub contact_id: Option<Id>,
    pub description: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    pub type_id: Option<Id>,
    pub state_id: Option<Id>,
    pub origin_id: Option<Id>,
    pub budget: Option<f64>,
    pub duration_months: Option<f64>,
    pub planned_days: Option<f64>,
    pub start_date: Option<String>,
    pub location: Option<String>,
    pub owner_id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NeedFilter {
    pub q: Option<String>,
    pub company_id: Option<Id>,
    pub contact_id: Option<Id>,
    pub owner_id: Option<Id>,
    pub state_id: Option<Id>,
    /// `open`, `won`, `lost` ou `standby`.
    pub state_category: Option<String>,
    pub type_id: Option<Id>,
    pub start_from: Option<String>,
    pub start_to: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StateChange {
    pub state_id: Id,
}

// ──────────────────────────── Ressources ────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Candidate,
    Consultant,
}

impl ResourceKind {
    pub fn code(self) -> &'static str {
        match self {
            ResourceKind::Candidate => "candidate",
            ResourceKind::Consultant => "consultant",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "candidate" => Some(Self::Candidate),
            "consultant" => Some(Self::Consultant),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Resource {
    pub id: Id,
    pub kind: ResourceKind,
    pub first_name: String,
    pub last_name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub linkedin_url: Option<String>,
    pub skills: Vec<String>,
    pub experience_years: Option<f64>,
    pub experience: Option<String>,
    pub availability_date: Option<String>,
    pub availability_note: Option<String>,
    pub contract_type_id: Option<Id>,
    /// CJM saisi directement sur la ressource.
    pub daily_cost: Option<f64>,
    /// CJM retenu : contrat RH en cours, sinon CJM saisi.
    pub effective_daily_cost: Option<f64>,
    /// `contract` ou `manual`.
    pub daily_cost_source: Option<String>,
    pub notes: Option<String>,
    pub owner: Option<Named>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    /// Fin de la prestation en cours, si la ressource est en mission aujourd'hui.
    pub on_mission_until: Option<String>,
    pub on_mission: bool,
    pub open_positionings: i64,
}

impl Resource {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResourceInput {
    pub kind: Option<ResourceKind>,
    pub first_name: String,
    pub last_name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub linkedin_url: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    pub experience_years: Option<f64>,
    pub experience: Option<String>,
    pub availability_date: Option<String>,
    pub availability_note: Option<String>,
    pub contract_type_id: Option<Id>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
    pub owner_id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResourceFilter {
    pub q: Option<String>,
    pub kind: Option<ResourceKind>,
    pub contract_type_id: Option<Id>,
    /// `now` (disponible), `soon` (sous 30 jours) ou `mission` (en mission).
    pub availability: Option<String>,
    pub skill: Option<String>,
    pub location: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HrContract {
    pub id: Id,
    pub resource_id: Id,
    pub contract_type_id: Option<Id>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HrContractInput {
    pub contract_type_id: Option<Id>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceFile {
    pub id: Id,
    pub resource_id: Id,
    /// `cv`, `skills_file` (dossier de compétences) ou `other`.
    pub kind: String,
    pub filename: String,
    pub mime: String,
    pub size: i64,
    pub uploaded_at: String,
    #[serde(skip)]
    pub stored_name: String,
}

// ───────────────────────── Positionnements ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Positioning {
    pub id: Id,
    pub need: Named,
    pub company: Option<Named>,
    pub resource: Named,
    pub resource_kind: ResourceKind,
    pub resource_title: Option<String>,
    pub state_id: Id,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub daily_rate: Option<f64>,
    pub days: Option<f64>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
    pub project: Option<Named>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PositioningInput {
    pub need_id: Option<Id>,
    pub resource_id: Option<Id>,
    pub state_id: Option<Id>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub daily_rate: Option<f64>,
    pub days: Option<f64>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PositioningFilter {
    pub need_id: Option<Id>,
    pub resource_id: Option<Id>,
    pub state_category: Option<String>,
}

/// Résultat d'une opération qui peut proposer la création d'un projet.
#[derive(Debug, Clone, Serialize)]
pub struct PositioningSaved {
    pub positioning: Positioning,
    /// Vrai quand le positionnement vient de passer « gagné » sans projet associé.
    pub suggest_project: bool,
    /// Projet créé automatiquement selon les réglages.
    pub created_project: Option<Named>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConvertInput {
    /// `new` ou `existing`.
    pub mode: String,
    pub project_id: Option<Id>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchReason {
    /// `skills`, `availability`, `location`, `title`.
    pub kind: String,
    pub positive: bool,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub resource: Resource,
    pub score: u8,
    pub reasons: Vec<MatchReason>,
}

// ─────────────────────── Projets et prestations ───────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: Id,
    pub reference: Option<String>,
    pub title: String,
    pub company: Option<Named>,
    pub contact: Option<Named>,
    pub type_id: Option<Id>,
    pub state_id: Option<Id>,
    pub manager: Option<Named>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub forecast_end_date: Option<String>,
    /// Vrai quand les dates sont calculées à partir des prestations.
    pub dates_from_deliveries: bool,
    pub need: Option<Named>,
    pub notes: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub resources: Vec<Named>,
    pub deliveries_count: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectInput {
    pub reference: Option<String>,
    pub title: String,
    pub company_id: Option<Id>,
    pub contact_id: Option<Id>,
    pub type_id: Option<Id>,
    pub state_id: Option<Id>,
    pub manager_id: Option<Id>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectFilter {
    pub q: Option<String>,
    pub company_id: Option<Id>,
    pub resource_id: Option<Id>,
    pub manager_id: Option<Id>,
    pub state_id: Option<Id>,
    pub type_id: Option<Id>,
    /// Projets actifs sur la période [period_from, period_to].
    pub period_from: Option<String>,
    pub period_to: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Delivery {
    pub id: Id,
    pub project: Named,
    pub company: Option<Named>,
    pub resource: Option<Named>,
    pub type_id: Option<Id>,
    pub title: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub daily_rate: Option<f64>,
    pub sold_days: Option<f64>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
    pub extensions: Vec<Extension>,
    /// Fin en tenant compte des prolongations signées.
    pub effective_end_date: Option<String>,
    /// Fin en tenant compte de toutes les prolongations.
    pub forecast_end_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeliveryInput {
    pub resource_id: Option<Id>,
    pub type_id: Option<Id>,
    pub title: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub daily_rate: Option<f64>,
    pub sold_days: Option<f64>,
    pub daily_cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeliveryFilter {
    pub project_id: Option<Id>,
    pub resource_id: Option<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionStatus {
    Signed,
    Forecast,
}

impl ExtensionStatus {
    pub fn code(self) -> &'static str {
        match self {
            ExtensionStatus::Signed => "signed",
            ExtensionStatus::Forecast => "forecast",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "signed" => Some(Self::Signed),
            "forecast" => Some(Self::Forecast),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExtensionStatus::Signed => "signée",
            ExtensionStatus::Forecast => "prévisionnelle",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Extension {
    pub id: Id,
    pub delivery_id: Id,
    pub status: ExtensionStatus,
    pub start_date: String,
    pub end_date: String,
    pub days: Option<f64>,
    pub daily_rate: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtensionInput {
    pub status: Option<ExtensionStatus>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub days: Option<f64>,
    pub daily_rate: Option<f64>,
    pub notes: Option<String>,
}

// ─────────────────────────── Historique ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Company,
    Contact,
    Need,
    Positioning,
    Resource,
    Project,
    Delivery,
}

impl EntityKind {
    pub fn code(self) -> &'static str {
        match self {
            EntityKind::Company => "company",
            EntityKind::Contact => "contact",
            EntityKind::Need => "need",
            EntityKind::Positioning => "positioning",
            EntityKind::Resource => "resource",
            EntityKind::Project => "project",
            EntityKind::Delivery => "delivery",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        use EntityKind::*;
        [Company, Contact, Need, Positioning, Resource, Project, Delivery].into_iter().find(|k| k.code() == code)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: Id,
    pub entity: EntityKind,
    pub entity_id: Id,
    pub entity_label: String,
    /// `created`, `updated`, `state_changed`, `archived`, `restored`, `imported`, `converted`, `deleted`.
    pub action: String,
    pub changes: Vec<FieldChange>,
    /// Objet de navigation (ex. le besoin d'un positionnement).
    pub link_entity: Option<EntityKind>,
    pub link_id: Option<Id>,
    pub user: Option<Named>,
    pub at: String,
}

// ───────────────────── Tableau de bord et recherche ─────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct DashboardCounts {
    pub companies: i64,
    pub contacts: i64,
    pub open_needs: i64,
    pub resources: i64,
    pub candidates: i64,
    pub consultants: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dashboard {
    pub counts: DashboardCounts,
    pub recent: Vec<HistoryEntry>,
    pub reminders: Vec<Activity>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub kind: EntityKind,
    pub id: Id,
    pub title: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveInput {
    pub archived: bool,
}
