//! Normalisation de texte pour la recherche et la détection de doublons.

/// Minuscules sans accents : « Société Générale » → « societe generale ».
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => out.push('a'),
            'ç' | 'Ç' => out.push('c'),
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => out.push('e'),
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => out.push('i'),
            'ñ' | 'Ñ' => out.push('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => out.push('o'),
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => out.push('u'),
            'ý' | 'ÿ' | 'Ý' => out.push('y'),
            'œ' | 'Œ' => out.push_str("oe"),
            'æ' | 'Æ' => out.push_str("ae"),
            '’' | '\'' => out.push('\''),
            _ => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// Forme compacte (lettres et chiffres uniquement) : « Vue.js » → « vuejs ».
pub fn compact(s: &str) -> String {
    fold(s).chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Supprime les espaces superflus ; une chaîne vide devient `None`.
pub fn clean(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Nettoie une chaîne obligatoire.
pub fn clean_required(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nettoie une liste de mots-clés : supprime vides et doublons, conserve l'ordre.
pub fn clean_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    tags.into_iter()
        .map(|t| clean_required(&t))
        .filter(|t| !t.is_empty() && seen.insert(compact(t)))
        .collect()
}

/// Échappe une saisie pour un motif `LIKE ... ESCAPE '\'`.
pub fn like_pattern(q: &str) -> String {
    let folded = fold(q.trim());
    let escaped = folded.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

/// Normalise une URL de profil LinkedIn pour comparer deux profils.
pub fn linkedin_key(url: &str) -> Option<String> {
    let lower = url.trim().to_lowercase();
    let idx = lower.find("linkedin.com/in/")?;
    let slug = lower[idx + "linkedin.com/in/".len()..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_string();
    if slug.is_empty() { None } else { Some(format!("linkedin.com/in/{slug}")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_removes_accents() {
        assert_eq!(fold("Société Générale"), "societe generale");
        assert_eq!(compact("Vue.js"), "vuejs");
    }

    #[test]
    fn linkedin_key_ignores_query_and_trailing_slash() {
        assert_eq!(
            linkedin_key("https://www.linkedin.com/in/Jean-Dupont/?trk=x").as_deref(),
            Some("linkedin.com/in/jean-dupont")
        );
        assert_eq!(linkedin_key("https://example.com"), None);
    }

    #[test]
    fn tags_are_deduplicated() {
        let tags = clean_tags(vec!["Rust".into(), " rust ".into(), "".into(), "Vue.js".into()]);
        assert_eq!(tags, vec!["Rust".to_string(), "Vue.js".to_string()]);
    }
}
