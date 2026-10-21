//! Rôles et permissions.
//!
//! Une permission est un couple (domaine, action). Les rôles sont modifiables,
//! sauf le rôle administrateur qui a toujours tous les droits.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Crm,
    Needs,
    Positionings,
    Resources,
    Costs,
    Projects,
    Settings,
    Users,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Read,
    Create,
    Update,
    Delete,
}

impl Area {
    pub const ALL: [Area; 8] = [
        Area::Crm,
        Area::Needs,
        Area::Positionings,
        Area::Resources,
        Area::Costs,
        Area::Projects,
        Area::Settings,
        Area::Users,
    ];

    /// Actions qui ont un sens pour ce domaine.
    pub fn actions(self) -> &'static [Action] {
        use Action::*;
        match self {
            Area::Costs | Area::Settings => &[Read, Update],
            _ => &[Read, Create, Update, Delete],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Area::Crm => "CRM (sociétés, contacts, activités)",
            Area::Needs => "Besoins",
            Area::Positionings => "Positionnements",
            Area::Resources => "Ressources (candidats et consultants)",
            Area::Costs => "Coûts et contrats RH (CJM)",
            Area::Projects => "Projets et prestations",
            Area::Settings => "Réglages et paramètres métier",
            Area::Users => "Utilisateurs et rôles",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Permissions(pub BTreeMap<Area, BTreeSet<Action>>);

impl Permissions {
    pub fn all() -> Self {
        Self(Area::ALL.into_iter().map(|a| (a, a.actions().iter().copied().collect())).collect())
    }

    pub fn can(&self, area: Area, action: Action) -> bool {
        self.0.get(&area).is_some_and(|set| set.contains(&action))
    }

    /// Supprime les actions sans objet et ajoute la lecture dès qu'une écriture est accordée.
    pub fn normalized(mut self) -> Self {
        for (area, set) in self.0.iter_mut() {
            set.retain(|a| area.actions().contains(a));
            if !set.is_empty() {
                set.insert(Action::Read);
            }
        }
        self.0.retain(|_, set| !set.is_empty());
        self
    }

    fn with(mut self, area: Area, actions: &[Action]) -> Self {
        self.0.insert(area, actions.iter().copied().collect());
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AreaMeta {
    pub area: Area,
    pub label: &'static str,
    pub actions: &'static [Action],
}

pub fn areas_meta() -> Vec<AreaMeta> {
    Area::ALL.into_iter().map(|area| AreaMeta { area, label: area.label(), actions: area.actions() }).collect()
}

pub struct DefaultRole {
    pub name: &'static str,
    pub description: &'static str,
    pub is_admin: bool,
    pub permissions: Permissions,
}

/// Rôles créés à l'installation ; l'administrateur peut les adapter.
pub fn default_roles() -> Vec<DefaultRole> {
    use Action::*;
    const CRUD: &[Action] = &[Read, Create, Update, Delete];
    const RCU: &[Action] = &[Read, Create, Update];
    const RU: &[Action] = &[Read, Update];
    const R: &[Action] = &[Read];
    vec![
        DefaultRole {
            name: "Administrateur",
            description: "Tous les droits, y compris les réglages et les utilisateurs.",
            is_admin: true,
            permissions: Permissions::all(),
        },
        DefaultRole {
            name: "Commercial",
            description: "Gère la relation client, les besoins et les positionnements.",
            is_admin: false,
            permissions: Permissions::default()
                .with(Area::Crm, CRUD)
                .with(Area::Needs, CRUD)
                .with(Area::Positionings, CRUD)
                .with(Area::Resources, R)
                .with(Area::Costs, R)
                .with(Area::Projects, RCU)
                .with(Area::Settings, R),
        },
        DefaultRole {
            name: "Responsable des ressources",
            description: "Gère les candidats, les consultants et leurs contrats.",
            is_admin: false,
            permissions: Permissions::default()
                .with(Area::Crm, R)
                .with(Area::Needs, RU)
                .with(Area::Positionings, CRUD)
                .with(Area::Resources, CRUD)
                .with(Area::Costs, RU)
                .with(Area::Projects, RU)
                .with(Area::Settings, R),
        },
        DefaultRole {
            name: "Lecture seule",
            description: "Consulte les informations sans les modifier ; ne voit pas les coûts.",
            is_admin: false,
            permissions: Permissions::default()
                .with(Area::Crm, R)
                .with(Area::Needs, R)
                .with(Area::Positionings, R)
                .with(Area::Resources, R)
                .with(Area::Projects, R),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_implies_read_and_invalid_actions_are_dropped() {
        let p = Permissions::default()
            .with(Area::Crm, &[Action::Create])
            .with(Area::Costs, &[Action::Delete])
            .normalized();
        assert!(p.can(Area::Crm, Action::Read));
        assert!(!p.can(Area::Costs, Action::Read));
    }
}
