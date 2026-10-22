//! Validations communes et règles de calcul (dates de projet, CJM retenu).

use chrono::NaiveDate;

use crate::error::Errors;
use crate::model::ExtensionStatus;
use crate::settings::Settings;

pub fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// Vérifie le format `AAAA-MM-JJ` d'une date facultative.
pub fn check_date(errors: &mut Errors, field: &str, value: Option<&str>) {
    if let Some(v) = value
        && parse_date(v).is_none()
    {
        errors.add(field, "Date invalide (format attendu : AAAA-MM-JJ).");
    }
}

/// Vérifie que la fin n'est pas avant le début quand les deux sont connus.
pub fn check_range(errors: &mut Errors, end_field: &str, start: Option<&str>, end: Option<&str>) {
    if let (Some(s), Some(e)) = (start.and_then(parse_date), end.and_then(parse_date))
        && e < s
    {
        errors.add(end_field, "La date de fin doit être postérieure à la date de début.");
    }
}

pub fn check_amount(errors: &mut Errors, field: &str, value: Option<f64>) {
    if let Some(v) = value
        && (!v.is_finite() || v < 0.0)
    {
        errors.add(field, "Indiquez un nombre positif.");
    }
}

pub fn check_email(errors: &mut Errors, field: &str, value: Option<&str>) {
    if let Some(v) = value {
        let valid = v.split_once('@').is_some_and(|(user, domain)| {
            !user.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
        }) && !v.contains(char::is_whitespace);
        if !valid {
            errors.add(field, "Adresse e-mail invalide.");
        }
    }
}

/// Signale les champs rendus obligatoires dans les réglages et laissés vides.
pub fn check_required(errors: &mut Errors, settings: &Settings, entity: &str, present: &[(&str, bool)]) {
    for (field, is_present) in present {
        if !is_present && settings.is_required(entity, field) {
            errors.add(*field, "Ce champ est obligatoire selon les réglages.");
        }
    }
}

/// Fin d'une prestation avec ses prolongations : (signée, prévisionnelle).
///
/// Une prestation sans date de fin reste ouverte : ses fins calculées sont inconnues.
pub fn delivery_ends(end: Option<&str>, extensions: &[(ExtensionStatus, &str)]) -> (Option<String>, Option<String>) {
    let Some(end) = end else { return (None, None) };
    let signed = extensions
        .iter()
        .filter(|(s, _)| *s == ExtensionStatus::Signed)
        .map(|(_, e)| *e)
        .chain(std::iter::once(end))
        .max()
        .map(str::to_string);
    let forecast = extensions.iter().map(|(_, e)| *e).chain(std::iter::once(end)).max().map(str::to_string);
    (signed, forecast)
}

/// Une prestation vue pour le calcul des dates du projet.
pub struct DeliverySpan<'a> {
    pub start: &'a str,
    pub effective_end: Option<&'a str>,
    pub forecast_end: Option<&'a str>,
}

/// Dates d'un projet calculées à partir de ses prestations : (début, fin, fin prévisionnelle).
pub fn project_dates(spans: &[DeliverySpan]) -> (Option<String>, Option<String>, Option<String>) {
    if spans.is_empty() {
        return (None, None, None);
    }
    // Une seule prestation sans fin connue rend la fin du projet inconnue.
    fn latest<'a>(ends: impl Iterator<Item = Option<&'a str>>) -> Option<String> {
        let ends: Option<Vec<&str>> = ends.collect();
        ends.and_then(|e| e.into_iter().max().map(str::to_string))
    }
    let start = spans.iter().map(|s| s.start).min().map(str::to_string);
    (start, latest(spans.iter().map(|s| s.effective_end)), latest(spans.iter().map(|s| s.forecast_end)))
}

/// Une prolongation commence après la fin initiale de la prestation.
pub fn check_extension(errors: &mut Errors, delivery_end: Option<&str>, start: &str, end: &str) {
    check_range(errors, "end_date", Some(start), Some(end));
    match delivery_end {
        None => errors.add("start_date", "Renseignez d'abord la date de fin de la prestation."),
        Some(d) if start <= d => {
            errors.add("start_date", "La prolongation doit commencer après la fin initiale de la prestation.")
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_extension_moves_end_forecast_only_moves_forecast() {
        let ext = [(ExtensionStatus::Signed, "2026-06-30"), (ExtensionStatus::Forecast, "2026-12-31")];
        let (signed, forecast) = delivery_ends(Some("2026-03-31"), &ext);
        assert_eq!(signed.as_deref(), Some("2026-06-30"));
        assert_eq!(forecast.as_deref(), Some("2026-12-31"));
    }

    #[test]
    fn project_dates_follow_deliveries_and_open_end_stays_unknown() {
        let spans = [
            DeliverySpan { start: "2026-02-01", effective_end: Some("2026-05-31"), forecast_end: Some("2026-08-31") },
            DeliverySpan { start: "2026-01-15", effective_end: Some("2026-03-31"), forecast_end: Some("2026-03-31") },
        ];
        let (s, e, f) = project_dates(&spans);
        assert_eq!((s.as_deref(), e.as_deref(), f.as_deref()), (Some("2026-01-15"), Some("2026-05-31"), Some("2026-08-31")));

        let open = [DeliverySpan { start: "2026-01-01", effective_end: None, forecast_end: None }];
        assert_eq!(project_dates(&open).1, None);
    }
}
