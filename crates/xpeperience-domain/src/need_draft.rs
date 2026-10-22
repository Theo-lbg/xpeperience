//! Préremplissage d'un besoin à partir d'un texte collé (annonce, e-mail du client…).
//!
//! L'extraction repose sur des règles simples et locales : aucune donnée ne quitte l'application.
//! Elle ne fait que proposer des valeurs, que l'utilisateur relit avant d'enregistrer.

use chrono::NaiveDate;

use crate::model::NeedDraft;
use crate::text::fold;

const MAX_TITLE: usize = 120;
const MAX_DESCRIPTION: usize = 8000;

/// (nom affiché, forme repliée recherchée).
const SKILLS: &[(&str, &str)] = &[
    ("Rust", "rust"), ("Java", "java"), ("JavaScript", "javascript"), ("TypeScript", "typescript"),
    ("Python", "python"), ("PHP", "php"), ("C#", "c#"), ("C++", "c++"), ("Go", "golang"), ("Kotlin", "kotlin"),
    ("Swift", "swift"), ("Scala", "scala"), ("Ruby", "ruby"), ("Dart", "dart"), ("COBOL", "cobol"),
    ("Spring", "spring"), ("Spring Boot", "spring boot"), ("Symfony", "symfony"), ("Laravel", "laravel"),
    (".NET", ".net"), ("Django", "django"), ("Flask", "flask"), ("FastAPI", "fastapi"),
    ("Node.js", "node.js"), ("Node.js", "nodejs"), ("Express", "express.js"), ("NestJS", "nestjs"),
    ("React", "react"), ("React Native", "react native"), ("Next.js", "next.js"), ("Angular", "angular"),
    ("Vue.js", "vue.js"), ("Vue.js", "vuejs"), ("Nuxt", "nuxt"), ("Svelte", "svelte"), ("Flutter", "flutter"),
    ("HTML", "html"), ("CSS", "css"), ("SQL", "sql"), ("PostgreSQL", "postgresql"), ("PostgreSQL", "postgres"),
    ("MySQL", "mysql"), ("Oracle", "oracle"), ("SQL Server", "sql server"), ("MongoDB", "mongodb"),
    ("Redis", "redis"), ("Elasticsearch", "elasticsearch"), ("Kafka", "kafka"), ("RabbitMQ", "rabbitmq"),
    ("Docker", "docker"), ("Kubernetes", "kubernetes"), ("Terraform", "terraform"), ("Ansible", "ansible"),
    ("Jenkins", "jenkins"), ("GitLab CI", "gitlab ci"), ("Git", "git"), ("Linux", "linux"),
    ("AWS", "aws"), ("Azure", "azure"), ("GCP", "gcp"), ("DevOps", "devops"), ("CI/CD", "ci/cd"),
    ("Power BI", "power bi"), ("Tableau", "tableau"), ("Talend", "talend"), ("Spark", "spark"),
    ("Hadoop", "hadoop"), ("Snowflake", "snowflake"), ("Databricks", "databricks"), ("Airflow", "airflow"),
    ("Machine Learning", "machine learning"), ("Data Science", "data science"), ("SAP", "sap"),
    ("Salesforce", "salesforce"), ("ServiceNow", "servicenow"), ("Selenium", "selenium"), ("Cypress", "cypress"),
    ("JIRA", "jira"), ("Scrum", "scrum"), ("Agile", "agile"), ("Cybersécurité", "cybersecurite"),
    ("Figma", "figma"), ("GraphQL", "graphql"), ("REST", "rest"), ("Microservices", "microservices"),
    ("ITIL", "itil"), ("VMware", "vmware"), ("PowerShell", "powershell"), ("Bash", "bash"),
];

const CITIES: &[&str] = &[
    "Paris", "Lyon", "Lille", "Nantes", "Bordeaux", "Toulouse", "Marseille", "Rennes", "Strasbourg", "Nice",
    "Montpellier", "Grenoble", "Niort", "Nancy", "Metz", "Rouen", "Angers", "Tours", "Dijon", "Brest", "Sophia Antipolis",
    "La Défense", "Saint-Denis", "Boulogne-Billancourt", "Issy-les-Moulineaux", "Levallois-Perret", "Courbevoie",
];

const MONTHS: &[(&str, u32)] = &[
    ("janvier", 1), ("fevrier", 2), ("mars", 3), ("avril", 4), ("mai", 5), ("juin", 6), ("juillet", 7),
    ("aout", 8), ("septembre", 9), ("octobre", 10), ("novembre", 11), ("decembre", 12),
];

const GREETINGS: &[&str] = &["bonjour", "bonsoir", "hello", "salut", "madame", "monsieur", "cher", "chere", "hi ", "coucou"];

const TITLE_LABELS: &[&str] = &["objet", "subject", "titre", "intitule", "poste", "profil recherche", "profil"];
const LOCATION_LABELS: &[&str] = &["lieu", "localisation", "location", "ville", "site"];

fn is_bullet(c: char) -> bool {
    matches!(c, '-' | '*' | '•' | '–' | '·' | '>' | '#') || c.is_whitespace()
}

/// Valeur d'une ligne `Étiquette : valeur` dont l'étiquette commence par l'un des libellés.
fn labelled(lines: &[&str], labels: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        let (head, value) = line.split_once(':')?;
        let head = fold(head);
        let head = head.trim_matches(is_bullet);
        let value = value.trim().trim_matches(|c: char| c == '*' || c == '_').trim();
        (!value.is_empty() && labels.iter().any(|l| head == *l || head.starts_with(&format!("{l} ")))).then(|| value.to_string())
    })
}

fn strip_reply_prefix(mut s: &str) -> &str {
    loop {
        let folded = fold(s);
        let t = folded.trim_start();
        let skip = ["re :", "re:", "tr :", "tr:", "fwd :", "fwd:", "fw :", "fw:"].iter().find(|p| t.starts_with(**p)).map(|p| p.len());
        match skip {
            // Les préfixes sont en ASCII : leur longueur en octets est la même avant et après repli.
            Some(n) => s = s[s.len() - s.trim_start().len() + n..].trim_start(),
            None => return s.trim(),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    cut.trim_end().to_string()
}

fn title_of(lines: &[&str]) -> String {
    let raw = labelled(lines, TITLE_LABELS).or_else(|| {
        lines
            .iter()
            .map(|l| l.trim_matches(is_bullet))
            .find(|l| {
                let f = fold(l);
                !l.is_empty() && !l.contains('@') && !GREETINGS.iter().any(|g| f.starts_with(g)) && !f.ends_with(':')
            })
            .map(str::to_string)
    });
    raw.map(|t| truncate(strip_reply_prefix(&t), MAX_TITLE)).unwrap_or_default()
}

/// Découpe en nombres (avec décimales si demandé) et en mots ; tout le reste sert de séparateur.
fn tokens(text: &str, decimals: bool) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || (decimals && matches!(chars[i], ',' | '.') && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()))
            {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else if c.is_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else {
            i += 1;
        }
    }
    out
}

fn number(t: &str) -> Option<f64> {
    t.replace(',', ".").parse().ok()
}

fn month_of(t: &str) -> Option<u32> {
    MONTHS.iter().find(|(m, _)| *m == t).map(|(_, n)| *n)
}

fn iso(y: i32, m: u32, d: u32) -> Option<String> {
    NaiveDate::from_ymd_opt(y, m, d).map(|d| d.format("%Y-%m-%d").to_string())
}

fn year(t: &str) -> Option<i32> {
    let n: i32 = t.parse().ok()?;
    match t.len() {
        4 if (2000..=2100).contains(&n) => Some(n),
        2 => Some(2000 + n),
        _ => None,
    }
}

/// Première date reconnue dans la ligne : `12/03/2027`, `2027-03-12`, `1er mars 2027`, `mars 2027`.
fn date_in(line: &str) -> Option<String> {
    let t = tokens(&fold(line), false);
    let t: Vec<&str> = t.iter().map(String::as_str).filter(|w| *w != "er").collect();
    for i in 0..t.len() {
        if i + 2 < t.len() {
            let (a, b, c) = (t[i], t[i + 1], t[i + 2]);
            if a.len() == 4 && let (Some(y), Ok(m), Ok(d)) = (year(a), b.parse::<u32>(), c.parse::<u32>()) {
                if let Some(v) = iso(y, m, d) {
                    return Some(v);
                }
            }
            if a.len() <= 2 && let (Ok(d), Some(y)) = (a.parse::<u32>(), year(c)) {
                let m = b.parse::<u32>().ok().or_else(|| month_of(b));
                if let Some(v) = m.and_then(|m| iso(y, m, d)) {
                    return Some(v);
                }
            }
        }
        if i + 1 < t.len() && let (Some(m), Some(y)) = (month_of(t[i]), year(t[i + 1])) {
            return iso(y, m, 1);
        }
    }
    None
}

fn start_date(lines: &[&str]) -> Option<String> {
    const MARKERS: &[&str] = &["demarrage", "debut", "a partir du", "des le", "start", "disponible le", "prise de poste"];
    lines.iter().filter(|l| MARKERS.iter().any(|m| fold(l).contains(m))).find_map(|l| date_in(l))
}

/// Durée en mois (`6 mois`, `1 an`) et nombre de jours (`120 jours`) annoncés.
fn quantities(lines: &[&str]) -> (Option<f64>, Option<f64>) {
    let mut months = None;
    let mut days = None;
    for line in lines {
        let folded = fold(line);
        let priced = ["€", "tjm", "eur", "tarif", "teletravail", "remote", "sur site"].iter().any(|w| folded.contains(w));
        let duration_line = folded.contains("duree");
        let t = tokens(&folded, true);
        for w in t.windows(2) {
            let Some(n) = number(&w[0]) else { continue };
            let unit = w[1].as_str();
            if months.is_none() && unit == "mois" {
                months = Some(n);
            } else if months.is_none() && duration_line && matches!(unit, "an" | "ans") {
                months = Some(n * 12.0);
            } else if days.is_none() && !priced && matches!(unit, "jour" | "jours" | "jh" | "j") && n > 0.0 {
                days = Some(n);
            }
        }
    }
    (months, days)
}

fn location(lines: &[&str], folded_text: &str) -> Option<String> {
    if let Some(v) = labelled(lines, LOCATION_LABELS) {
        return Some(truncate(&v, 80));
    }
    CITIES
        .iter()
        .filter_map(|c| find_word(folded_text, &fold(c)).map(|pos| (pos, *c)))
        .min_by_key(|(pos, _)| *pos)
        .map(|(_, c)| c.to_string())
}

/// Position de `needle` dans `hay` quand il n'est pas collé à une lettre ou un chiffre.
fn find_word(hay: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(p) = hay[from..].find(needle) {
        let start = from + p;
        let end = start + needle.len();
        let before_ok = hay[..start].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let after_ok = hay[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return Some(start);
        }
        from = start + needle.len().max(1);
    }
    None
}

fn skills(folded_text: &str) -> Vec<String> {
    let mut found: Vec<(usize, &str)> = SKILLS
        .iter()
        .filter_map(|(name, key)| find_word(folded_text, key).map(|pos| (pos, *name)))
        .collect();
    found.sort_by_key(|(pos, _)| *pos);
    let mut out: Vec<String> = Vec::new();
    for (_, name) in found {
        if !out.iter().any(|s| s == name) {
            out.push(name.to_string());
        }
    }
    // « Spring » est redondant quand « Spring Boot » est cité ; idem pour « React ».
    for (long, short) in [("Spring Boot", "Spring"), ("React Native", "React")] {
        if out.iter().any(|s| s == long) && !find_word(&folded_text.replace(&fold(long), ""), &fold(short)).is_some() {
            out.retain(|s| s != short);
        }
    }
    out
}

fn emails(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | ',' | ';' | '(' | ')' | '"')) {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if let Some((local, domain)) = w.split_once('@')
            && !local.is_empty()
            && domain.contains('.')
            && !out.iter().any(|e| e.eq_ignore_ascii_case(w))
        {
            out.push(w.to_string());
        }
    }
    out
}

/// Propose les champs d'un besoin à partir d'un texte libre.
pub fn extract(text: &str) -> NeedDraft {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let folded = fold(&text);
    let (duration_months, planned_days) = quantities(&lines);
    NeedDraft {
        title: title_of(&lines),
        description: truncate(&text, MAX_DESCRIPTION),
        skills: skills(&folded),
        location: location(&lines, &folded),
        start_date: start_date(&lines),
        duration_months,
        planned_days,
        emails: emails(&text),
        contact: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Bonjour,\n\nObjet : TR: Développeur Rust confirmé H/F\n\nVoici notre besoin :\n- Lieu : Lyon (2 jours de télétravail)\n- Démarrage : 1er mars 2027\n- Durée : 6 mois renouvelables\n- Compétences : Rust, Vue.js, PostgreSQL, Docker\n- TJM : 650 € / jour\n\nCordialement,\nMarie Exemple <marie.exemple@client.test>";

    #[test]
    fn extracts_labelled_fields() {
        let d = extract(SAMPLE);
        assert_eq!(d.title, "Développeur Rust confirmé H/F");
        assert_eq!(d.location.as_deref(), Some("Lyon (2 jours de télétravail)"));
        assert_eq!(d.start_date.as_deref(), Some("2027-03-01"));
        assert_eq!(d.duration_months, Some(6.0));
        assert_eq!(d.skills, ["Rust", "Vue.js", "PostgreSQL", "Docker"]);
        assert_eq!(d.emails, ["marie.exemple@client.test"]);
    }

    #[test]
    fn days_ignore_prices_and_telework() {
        let d = extract("Consultant Java\nTJM 600 € / jour\nCharge : 120 jours\nDébut : 15/09/2027\nParis");
        assert_eq!(d.planned_days, Some(120.0));
        assert_eq!(d.start_date.as_deref(), Some("2027-09-15"));
        assert_eq!(d.location.as_deref(), Some("Paris"));
        assert_eq!(d.skills, ["Java"]);
    }

    #[test]
    fn skills_respect_word_boundaries() {
        let d = extract("Profil JavaScript avec notions de MySQL.");
        assert_eq!(d.skills, ["JavaScript", "MySQL"]);
        let d = extract("Stack React Native et Spring Boot");
        assert_eq!(d.skills, ["React Native", "Spring Boot"]);
    }

    #[test]
    fn first_line_is_the_title_when_nothing_is_labelled() {
        assert_eq!(extract("Bonjour,\nArchitecte cloud AWS senior\nDurée 1 an").title, "Architecte cloud AWS senior");
        assert_eq!(extract("Durée : 1 an").duration_months, Some(12.0));
        assert_eq!(extract("").title, "");
    }
}
