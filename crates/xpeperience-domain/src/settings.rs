//! Réglages généraux et paramètres métier.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Errors, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub app_name: String,
    /// Langue et région utilisées pour les formats (ex. `fr-FR`).
    pub locale: String,
    /// `dd/MM/yyyy`, `yyyy-MM-dd`, `MM/dd/yyyy` ou `d MMM yyyy`.
    pub date_format: String,
    pub currency: String,
    pub reminders: ReminderSettings,
    pub business: BusinessSettings,
    pub extension: ExtensionSettings,
    /// Champs facultatifs rendus obligatoires, par type d'objet.
    pub required_fields: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ReminderSettings {
    pub enabled: bool,
    /// Échéance proposée par défaut pour une relance, en jours.
    pub default_follow_up_days: i64,
    /// Fenêtre des rappels « à venir » affichés au tableau de bord, en jours.
    pub upcoming_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BusinessSettings {
    /// Que faire quand un positionnement passe « gagné » : `ask`, `auto` ou `none`.
    pub positioning_won_action: String,
    /// Passer le besoin dans un état « gagné » quand un projet est créé à partir de lui.
    pub mark_need_won: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExtensionSettings {
    pub enabled: bool,
    pub allow_company_creation: bool,
    /// Ajouter une note « Importé depuis LinkedIn » sur la fiche du contact.
    pub add_import_note: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            app_name: "XpePerience".into(),
            locale: "fr-FR".into(),
            date_format: "dd/MM/yyyy".into(),
            currency: "EUR".into(),
            reminders: ReminderSettings::default(),
            business: BusinessSettings::default(),
            extension: ExtensionSettings::default(),
            required_fields: BTreeMap::new(),
        }
    }
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self { enabled: true, default_follow_up_days: 7, upcoming_days: 7 }
    }
}

impl Default for BusinessSettings {
    fn default() -> Self {
        Self { positioning_won_action: "ask".into(), mark_need_won: true }
    }
}

impl Default for ExtensionSettings {
    fn default() -> Self {
        Self { enabled: true, allow_company_creation: true, add_import_note: true }
    }
}

pub const LOCALES: &[(&str, &str)] =
    &[("fr-FR", "Français (France)"), ("fr-BE", "Français (Belgique)"), ("fr-CH", "Français (Suisse)"), ("en-GB", "English (UK)")];
pub const DATE_FORMATS: &[&str] = &["dd/MM/yyyy", "yyyy-MM-dd", "MM/dd/yyyy", "d MMM yyyy"];
pub const CURRENCIES: &[&str] = &["EUR", "CHF", "GBP", "USD"];
pub const WON_ACTIONS: &[&str] = &["ask", "auto", "none"];

/// Champ qu'un administrateur peut rendre obligatoire.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OptionalField {
    pub entity: &'static str,
    pub field: &'static str,
    pub label: &'static str,
}

const fn opt(entity: &'static str, field: &'static str, label: &'static str) -> OptionalField {
    OptionalField { entity, field, label }
}

/// Le CJM n'apparaît volontairement dans aucune entité : il reste toujours facultatif.
pub const OPTIONAL_FIELDS: &[OptionalField] = &[
    opt("company", "type_id", "Type"),
    opt("company", "sector_id", "Secteur"),
    opt("company", "owner_id", "Responsable commercial"),
    opt("company", "phone", "Téléphone"),
    opt("company", "email", "E-mail"),
    opt("company", "city", "Ville"),
    opt("contact", "first_name", "Prénom"),
    opt("contact", "company_id", "Société"),
    opt("contact", "job_title", "Poste"),
    opt("contact", "email", "E-mail"),
    opt("contact", "phone", "Téléphone"),
    opt("contact", "owner_id", "Responsable commercial"),
    opt("need", "description", "Description"),
    opt("need", "skills", "Compétences recherchées"),
    opt("need", "type_id", "Type"),
    opt("need", "origin_id", "Provenance"),
    opt("need", "budget", "Budget"),
    opt("need", "duration_months", "Durée"),
    opt("need", "planned_days", "Nombre de jours prévus"),
    opt("need", "start_date", "Date de démarrage"),
    opt("need", "owner_id", "Responsable commercial"),
    opt("resource", "title", "Métier"),
    opt("resource", "email", "E-mail"),
    opt("resource", "phone", "Téléphone"),
    opt("resource", "location", "Localisation"),
    opt("resource", "skills", "Compétences"),
    opt("resource", "availability_date", "Date de disponibilité"),
    opt("resource", "contract_type_id", "Type de contrat"),
    opt("project", "reference", "Référence"),
    opt("project", "contact_id", "Contact"),
    opt("project", "type_id", "Type"),
    opt("project", "manager_id", "Responsable / chef de projet"),
    opt("delivery", "resource_id", "Ressource affectée"),
    opt("delivery", "end_date", "Date de fin"),
    opt("delivery", "daily_rate", "Tarif journalier de vente"),
    opt("delivery", "sold_days", "Jours vendus"),
];

impl Settings {
    pub fn is_required(&self, entity: &str, field: &str) -> bool {
        self.required_fields.get(entity).is_some_and(|f| f.iter().any(|x| x == field))
    }

    /// Vérifie les valeurs et retire les champs obligatoires inconnus.
    pub fn validated(mut self) -> Result<Self> {
        let mut errors = Errors::new();
        self.app_name = self.app_name.trim().to_string();
        if self.app_name.is_empty() {
            errors.add("app_name", "Le nom de l'application est obligatoire.");
        }
        if !LOCALES.iter().any(|(code, _)| *code == self.locale) {
            errors.add("locale", "Langue non prise en charge.");
        }
        if !DATE_FORMATS.contains(&self.date_format.as_str()) {
            errors.add("date_format", "Format de date non pris en charge.");
        }
        if !CURRENCIES.contains(&self.currency.as_str()) {
            errors.add("currency", "Devise non prise en charge.");
        }
        if !WON_ACTIONS.contains(&self.business.positioning_won_action.as_str()) {
            errors.add("business.positioning_won_action", "Valeur inconnue.");
        }
        if !(0..=365).contains(&self.reminders.default_follow_up_days) {
            errors.add("reminders.default_follow_up_days", "Entre 0 et 365 jours.");
        }
        if !(1..=90).contains(&self.reminders.upcoming_days) {
            errors.add("reminders.upcoming_days", "Entre 1 et 90 jours.");
        }
        errors.finish()?;

        for (entity, fields) in self.required_fields.iter_mut() {
            fields.retain(|f| OPTIONAL_FIELDS.iter().any(|o| o.entity == entity && o.field == f));
            fields.sort();
            fields.dedup();
        }
        self.required_fields.retain(|_, f| !f.is_empty());
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_cost_can_never_be_required() {
        let mut s = Settings::default();
        s.required_fields.insert("delivery".into(), vec!["daily_cost".into(), "daily_rate".into()]);
        let s = s.validated().unwrap();
        assert!(!s.is_required("delivery", "daily_cost"));
        assert!(s.is_required("delivery", "daily_rate"));
    }
}
