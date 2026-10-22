//! Listes paramétrables (états, types, provenances…).
//!
//! Chaque élément porte un libellé libre et, pour certaines listes, une
//! catégorie stable sur laquelle reposent les règles métier.

use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefList {
    CompanyType,
    Sector,
    NeedState,
    NeedType,
    NeedOrigin,
    PositioningState,
    ProjectType,
    ProjectState,
    DeliveryType,
    ContractType,
}

/// Catégorie d'un élément : code stable et libellé affiché.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Category {
    pub code: &'static str,
    pub label: &'static str,
}

const fn cat(code: &'static str, label: &'static str) -> Category {
    Category { code, label }
}

impl RefList {
    pub const ALL: [RefList; 10] = [
        RefList::CompanyType,
        RefList::Sector,
        RefList::NeedState,
        RefList::NeedType,
        RefList::NeedOrigin,
        RefList::PositioningState,
        RefList::ProjectType,
        RefList::ProjectState,
        RefList::DeliveryType,
        RefList::ContractType,
    ];

    pub fn code(self) -> &'static str {
        match self {
            RefList::CompanyType => "company_type",
            RefList::Sector => "sector",
            RefList::NeedState => "need_state",
            RefList::NeedType => "need_type",
            RefList::NeedOrigin => "need_origin",
            RefList::PositioningState => "positioning_state",
            RefList::ProjectType => "project_type",
            RefList::ProjectState => "project_state",
            RefList::DeliveryType => "delivery_type",
            RefList::ContractType => "contract_type",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            RefList::CompanyType => "Types de société",
            RefList::Sector => "Secteurs d'activité",
            RefList::NeedState => "États des besoins",
            RefList::NeedType => "Types de besoin",
            RefList::NeedOrigin => "Provenances des besoins",
            RefList::PositioningState => "États des positionnements",
            RefList::ProjectType => "Types de projet",
            RefList::ProjectState => "États des projets",
            RefList::DeliveryType => "Types de prestation",
            RefList::ContractType => "Types de contrat",
        }
    }

    /// Catégories disponibles. Une liste sans catégorie n'a pas de règle métier.
    pub fn categories(self) -> &'static [Category] {
        const STATES: &[Category] =
            &[cat("open", "En cours"), cat("won", "Gagné"), cat("lost", "Perdu"), cat("standby", "En attente")];
        const POSITIONING: &[Category] = &[cat("open", "En cours"), cat("won", "Gagné"), cat("lost", "Perdu")];
        const NEED_TYPES: &[Category] = &[
            cat("regie", "Régie / assistance technique"),
            cat("forfait", "Forfait"),
            cat("recrutement", "Recrutement"),
            cat("produit", "Produit"),
        ];
        const PROJECT_TYPES: &[Category] = &[
            cat("regie", "Régie"),
            cat("forfait", "Forfait"),
            cat("cession", "Cession"),
            cat("interne", "Projet interne"),
            cat("groupement", "Groupement de prestations"),
            cat("produit", "Projet produit"),
        ];
        const PROJECT_STATES: &[Category] =
            &[cat("planned", "À démarrer"), cat("active", "En cours"), cat("done", "Terminé"), cat("cancelled", "Annulé")];
        const CONTRACTS: &[Category] = &[cat("internal", "Salarié"), cat("external", "Externe")];
        match self {
            RefList::NeedState => STATES,
            RefList::PositioningState => POSITIONING,
            RefList::NeedType => NEED_TYPES,
            RefList::ProjectType => PROJECT_TYPES,
            RefList::ProjectState => PROJECT_STATES,
            RefList::ContractType => CONTRACTS,
            _ => &[],
        }
    }

    pub fn has_categories(self) -> bool {
        !self.categories().is_empty()
    }

    pub fn is_valid_category(self, code: &str) -> bool {
        self.categories().iter().any(|c| c.code == code)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RefItem {
    pub id: Id,
    pub list: RefList,
    pub label: String,
    pub category: Option<String>,
    pub color: Option<String>,
    pub position: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefItemInput {
    pub label: String,
    pub category: Option<String>,
    pub color: Option<String>,
    pub active: Option<bool>,
}

/// Couleurs de pastille proposées dans l'interface.
pub const COLORS: &[&str] = &["green", "blue", "orange", "red", "purple", "teal", "yellow", "gray"];

/// Description d'une liste pour l'écran de réglages.
#[derive(Debug, Clone, Serialize)]
pub struct RefListMeta {
    pub list: RefList,
    pub label: &'static str,
    pub categories: &'static [Category],
}

pub fn lists_meta() -> Vec<RefListMeta> {
    RefList::ALL
        .into_iter()
        .map(|list| RefListMeta { list, label: list.label(), categories: list.categories() })
        .collect()
}

/// Valeurs proposées à la première installation. Ce sont des paramètres, pas des données métier.
pub fn defaults() -> Vec<(RefList, &'static str, Option<&'static str>, Option<&'static str>)> {
    use RefList::*;
    vec![
        (CompanyType, "Client", None, Some("green")),
        (CompanyType, "Prospect", None, Some("blue")),
        (CompanyType, "Partenaire", None, Some("purple")),
        (CompanyType, "Fournisseur", None, Some("gray")),
        (Sector, "Banque et assurance", None, None),
        (Sector, "Industrie", None, None),
        (Sector, "Distribution et commerce", None, None),
        (Sector, "Énergie", None, None),
        (Sector, "Santé", None, None),
        (Sector, "Secteur public", None, None),
        (Sector, "Télécoms et médias", None, None),
        (Sector, "Transport et logistique", None, None),
        (Sector, "Services", None, None),
        (Sector, "Autre", None, None),
        (NeedState, "Nouveau", Some("open"), Some("blue")),
        (NeedState, "En cours", Some("open"), Some("orange")),
        (NeedState, "Gagné", Some("won"), Some("green")),
        (NeedState, "Perdu", Some("lost"), Some("red")),
        (NeedState, "Reporté", Some("standby"), Some("gray")),
        (NeedType, "Régie / assistance technique", Some("regie"), Some("green")),
        (NeedOrigin, "Appel d'offres", None, None),
        (NeedOrigin, "Client existant", None, None),
        (NeedOrigin, "Prospection", None, None),
        (NeedOrigin, "Recommandation", None, None),
        (NeedOrigin, "Réseau / LinkedIn", None, None),
        (NeedOrigin, "Autre", None, None),
        (PositioningState, "À qualifier", Some("open"), Some("gray")),
        (PositioningState, "Proposé au client", Some("open"), Some("blue")),
        (PositioningState, "Entretien client", Some("open"), Some("orange")),
        (PositioningState, "Gagné", Some("won"), Some("green")),
        (PositioningState, "Perdu", Some("lost"), Some("red")),
        (PositioningState, "Abandonné", Some("lost"), Some("gray")),
        (ProjectType, "Régie", Some("regie"), Some("green")),
        (ProjectType, "Forfait", Some("forfait"), Some("blue")),
        (ProjectType, "Cession", Some("cession"), Some("orange")),
        (ProjectType, "Projet interne", Some("interne"), Some("gray")),
        (ProjectType, "Groupement de prestations", Some("groupement"), Some("purple")),
        (ProjectType, "Projet produit", Some("produit"), Some("teal")),
        (ProjectState, "À démarrer", Some("planned"), Some("blue")),
        (ProjectState, "En cours", Some("active"), Some("green")),
        (ProjectState, "Terminé", Some("done"), Some("gray")),
        (ProjectState, "Annulé", Some("cancelled"), Some("red")),
        (DeliveryType, "Assistance technique", None, None),
        (DeliveryType, "Expertise", None, None),
        (DeliveryType, "Pilotage", None, None),
        (DeliveryType, "Formation", None, None),
        (DeliveryType, "Autre", None, None),
        (ContractType, "CDI", Some("internal"), None),
        (ContractType, "CDD", Some("internal"), None),
        (ContractType, "Alternance / stage", Some("internal"), None),
        (ContractType, "Freelance", Some("external"), None),
        (ContractType, "Sous-traitance", Some("external"), None),
        (ContractType, "Portage salarial", Some("external"), None),
    ]
}

/// Correspondance type de besoin → type de projet lors de la conversion.
pub fn project_kind_for_need(need_kind: Option<&str>) -> &'static str {
    match need_kind {
        Some("forfait") => "forfait",
        Some("produit") => "produit",
        _ => "regie",
    }
}
