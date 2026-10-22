//! Suggestion de profils pour un besoin, avec l'explication de chaque critère.
//!
//! Pondération : compétences 60 %, disponibilité 25 %, localisation 15 %.

use chrono::Duration;

use crate::model::{MatchReason, Resource};
use crate::rules::parse_date;
use crate::text::{compact, fold};

pub struct Criteria<'a> {
    pub title: &'a str,
    pub skills: &'a [String],
    pub start_date: Option<&'a str>,
    pub location: Option<&'a str>,
}

const STOP_WORDS: &[&str] = &[
    "les", "des", "une", "pour", "avec", "dans", "sur", "par", "and", "the", "for", "h/f", "f/h", "mission", "besoin",
    "projet", "client", "senior", "junior", "confirme",
];

fn skill_matches(wanted: &str, have: &str) -> bool {
    wanted == have || (wanted.len() >= 3 && have.len() >= 3 && (have.contains(wanted) || wanted.contains(have)))
}

fn reason(kind: &str, positive: bool, text: String) -> MatchReason {
    MatchReason { kind: kind.into(), positive, text }
}

fn format_date(iso: &str) -> String {
    parse_date(iso).map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_else(|| iso.to_string())
}

/// Calcule un score de 0 à 100 et les raisons associées.
pub fn score(criteria: &Criteria, resource: &Resource, today: &str) -> (u8, Vec<MatchReason>) {
    let mut reasons = Vec::new();
    let have: Vec<String> = resource
        .skills
        .iter()
        .map(|s| compact(s))
        .chain(resource.title.iter().flat_map(|t| fold(t).split_whitespace().map(compact).collect::<Vec<_>>()))
        .filter(|s| !s.is_empty())
        .collect();

    // Compétences, ou mots du titre quand le besoin n'a pas de compétences.
    let (wanted, from_title): (Vec<String>, bool) = if criteria.skills.is_empty() {
        let words = fold(criteria.title)
            .split(|c: char| !c.is_alphanumeric() && c != '#' && c != '+')
            .filter(|w| w.len() >= 3 && !STOP_WORDS.contains(w))
            .map(str::to_string)
            .collect();
        (words, true)
    } else {
        (criteria.skills.to_vec(), false)
    };
    let skills_score = if wanted.is_empty() {
        0.0
    } else {
        let (matched, missing): (Vec<&String>, Vec<&String>) =
            wanted.iter().partition(|w| have.iter().any(|h| skill_matches(&compact(w), h)));
        if !matched.is_empty() {
            let list = matched.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            let text = if from_title {
                format!("Profil proche du titre du besoin : {list}")
            } else {
                format!("Compétences correspondantes ({}/{}) : {list}", matched.len(), wanted.len())
            };
            reasons.push(reason("skills", true, text));
        }
        if !missing.is_empty() && !from_title {
            let list = missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            reasons.push(reason("skills", false, format!("Compétences non trouvées : {list}")));
        }
        matched.len() as f64 / wanted.len() as f64
    };

    // Disponibilité par rapport au démarrage souhaité (ou à aujourd'hui).
    let target = criteria.start_date.unwrap_or(today);
    let available_from = resource.on_mission_until.as_deref().or(resource.availability_date.as_deref());
    let availability_score = match available_from {
        // Sans date ni mission en cours, la ressource est considérée disponible (comme dans les listes).
        None => {
            reasons.push(reason("availability", true, "Disponible dès maintenant".into()));
            1.0
        }
        Some(date) if date <= target => {
            let text = if resource.on_mission {
                format!("Fin de mission le {} : disponible pour le démarrage", format_date(date))
            } else if date <= today {
                "Disponible dès maintenant".into()
            } else {
                format!("Disponible le {}, avant le démarrage", format_date(date))
            };
            reasons.push(reason("availability", true, text));
            1.0
        }
        Some(date) => {
            let soon = match (parse_date(date), parse_date(target)) {
                (Some(d), Some(t)) => d <= t + Duration::days(30),
                _ => false,
            };
            let prefix = if resource.on_mission { "En mission jusqu'au" } else { "Disponible le" };
            if soon {
                reasons.push(reason("availability", true, format!("{prefix} {}, peu après le démarrage", format_date(date))));
                0.5
            } else {
                reasons.push(reason("availability", false, format!("{prefix} {}", format_date(date))));
                0.0
            }
        }
    };

    // Localisation.
    let location_score = match (criteria.location.map(fold), resource.location.as_deref().map(fold)) {
        (Some(want), Some(have)) if !want.is_empty() => {
            if have.contains(&want) || want.contains(&have) {
                reasons.push(reason("location", true, format!("Localisation : {}", resource.location.as_deref().unwrap_or_default())));
                1.0
            } else {
                reasons.push(reason(
                    "location",
                    false,
                    format!("Localisation différente : {}", resource.location.as_deref().unwrap_or_default()),
                ));
                0.0
            }
        }
        (Some(want), None) if !want.is_empty() => {
            reasons.push(reason("location", false, "Localisation non renseignée".into()));
            0.5
        }
        _ => 0.5,
    };

    let total = 60.0 * skills_score + 25.0 * availability_score + 15.0 * location_score;
    (total.round().clamp(0.0, 100.0) as u8, reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ResourceKind;

    fn resource(skills: &[&str], available: Option<&str>, location: Option<&str>) -> Resource {
        Resource {
            id: 1,
            kind: ResourceKind::Consultant,
            first_name: "Profil".into(),
            last_name: "Test".into(),
            title: Some("Développeur".into()),
            email: None,
            phone: None,
            location: location.map(Into::into),
            linkedin_url: None,
            skills: skills.iter().map(|s| s.to_string()).collect(),
            experience_years: None,
            experience: None,
            availability_date: available.map(Into::into),
            availability_note: None,
            contract_type_id: None,
            daily_cost: None,
            notes: None,
            owner: None,
            archived: false,
            created_at: String::new(),
            updated_at: String::new(),
            on_mission_until: None,
            on_mission: false,
            open_positionings: 0,
        }
    }

    #[test]
    fn full_match_scores_high_and_explains() {
        let skills = vec!["Rust".to_string(), "Vue.js".to_string()];
        let c = Criteria { title: "Dev", skills: &skills, start_date: Some("2026-11-01"), location: Some("Lille") };
        let (score, reasons) = score(&c, &resource(&["rust", "Vue"], Some("2026-10-01"), Some("Lille (59)")), "2026-10-21");
        assert_eq!(score, 100);
        assert!(reasons.iter().all(|r| r.positive));
    }

    #[test]
    fn missing_skills_and_late_availability_lower_the_score() {
        let skills = vec!["Java".to_string(), "Kafka".to_string()];
        let c = Criteria { title: "Dev", skills: &skills, start_date: Some("2026-11-01"), location: None };
        let (score, reasons) = score(&c, &resource(&["Java"], Some("2027-06-01"), None), "2026-10-21");
        assert_eq!(score, 38); // 60 × 0,5 + 0 + 15 × 0,5
        assert!(reasons.iter().any(|r| !r.positive && r.text.contains("Kafka")));
    }
}
