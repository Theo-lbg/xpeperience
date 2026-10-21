//! Serveur autonome de Boond by XPEHO (mode équipe, développement).
//!
//! Commandes :
//! - `boond-server`                 démarre le serveur ;
//! - `boond-server healthcheck`     sonde de santé (code de sortie 0 si le serveur répond) ;
//! - `boond-server backup <dossier>` sauvegarde cohérente de la base, des documents et du logo.
//!
//! Variables d'environnement :
//! - `BOOND_DATA_DIR`        dossier des données (défaut : dossier utilisateur) ;
//! - `BOOND_BIND`            adresse d'écoute (défaut `127.0.0.1` ; `0.0.0.0` dans Docker) ;
//! - `BOOND_PORT`            port (défaut 47821) ;
//! - `BOOND_PUBLIC_URL`      adresse publique, ex. `https://boond.exemple.fr` (obligatoire hors boucle locale) ;
//! - `BOOND_TRUST_PROXY`     `1` si le serveur est derrière un proxy de confiance (lit `X-Forwarded-For`) ;
//! - `BOOND_SETUP_CODE`      code demandé pour créer le premier administrateur (généré s'il est absent) ;
//! - `BOOND_ALLOWED_HOSTS`   hôtes supplémentaires acceptés (liste séparée par des virgules) ;
//! - `BOOND_ALLOW_INSECURE_HTTP=1` autorise l'écoute hors boucle locale sans adresse publique (réseau privé, essais).

use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use boond_http::{DEFAULT_PORT, ServerConfig, build_state, default_data_dir, generate_setup_code, serve_until};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn flag(name: &str) -> bool {
    matches!(env(name).as_deref(), Some("1" | "true" | "yes" | "oui"))
}

fn port() -> u16 {
    env("BOOND_PORT").and_then(|p| p.parse().ok()).unwrap_or(DEFAULT_PORT)
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}

/// Sonde de santé sans dépendance : requête HTTP minimale sur la boucle locale.
fn healthcheck() -> ! {
    let port = port();
    let ok = (|| -> std::io::Result<bool> {
        let mut s = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(3))?;
        s.set_read_timeout(Some(Duration::from_secs(3)))?;
        write!(s, "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")?;
        let mut out = String::new();
        s.read_to_string(&mut out)?;
        Ok(out.starts_with("HTTP/1.1 200"))
    })()
    .unwrap_or(false);
    std::process::exit(if ok { 0 } else { 1 })
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<u64> {
    std::fs::create_dir_all(to)?;
    let mut n = 0;
    if !from.is_dir() {
        return Ok(0);
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), to.join(entry.file_name()))?;
            n += 1;
        }
    }
    Ok(n)
}

fn backup(dest: PathBuf) {
    let data = default_data_dir();
    let db = data.join("boond.sqlite3");
    if !db.is_file() {
        fail(format!("Aucune base trouvée dans {}.", data.display()));
    }
    if dest.join("boond.sqlite3").exists() {
        fail(format!("{} contient déjà une sauvegarde : choisissez un autre dossier.", dest.display()));
    }
    std::fs::create_dir_all(&dest).unwrap_or_else(|e| fail(format!("Dossier de sauvegarde inaccessible : {e}")));
    let conn = boond_store::open(&db).unwrap_or_else(|e| fail(format!("Base inaccessible : {e}")));
    boond_store::backup_to(&conn, &dest.join("boond.sqlite3")).unwrap_or_else(|e| fail(format!("Sauvegarde impossible : {e}")));
    let docs = copy_dir(&data.join("documents"), &dest.join("documents")).unwrap_or_else(|e| fail(format!("Documents : {e}")));
    let brand = copy_dir(&data.join("branding"), &dest.join("branding")).unwrap_or_else(|e| fail(format!("Logo : {e}")));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o700));
        let _ = std::fs::set_permissions(dest.join("boond.sqlite3"), std::fs::Permissions::from_mode(0o600));
    }
    println!("Sauvegarde terminée dans {} (base, {docs} document(s), {brand} fichier(s) de marque).", dest.display());
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("signal SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    println!("Arrêt demandé : fin des requêtes en cours…");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("healthcheck") => healthcheck(),
        Some("backup") => return backup(args.get(1).map(PathBuf::from).unwrap_or_else(|| fail("Usage : boond-server backup <dossier>"))),
        Some(other) => fail(format!("Commande inconnue : {other}. Commandes : healthcheck, backup <dossier>.")),
        None => {}
    }

    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();

    let port = port();
    let bind = env("BOOND_BIND").unwrap_or_else(|| "127.0.0.1".into());
    let loopback = bind.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()) || bind == "localhost";
    let public_url = env("BOOND_PUBLIC_URL").map(|u| u.trim_end_matches('/').to_string());

    // Écouter hors de cet ordinateur sans adresse publique (donc sans HTTPS) est refusé par défaut.
    if !loopback && public_url.is_none() && !flag("BOOND_ALLOW_INSECURE_HTTP") {
        fail(
            "Écoute sur une adresse non locale sans BOOND_PUBLIC_URL.\n\
             Placez le serveur derrière un proxy HTTPS et renseignez BOOND_PUBLIC_URL=https://votre-domaine,\n\
             ou définissez BOOND_ALLOW_INSECURE_HTTP=1 pour un essai sur un réseau privé (mots de passe non chiffrés).",
        );
    }
    if let Some(url) = &public_url
        && url.starts_with("http://")
        && !flag("BOOND_ALLOW_INSECURE_HTTP")
        && !url.contains("127.0.0.1")
        && !url.contains("localhost")
    {
        eprintln!("Avertissement : BOOND_PUBLIC_URL n'est pas en HTTPS. Les mots de passe circuleront en clair.");
    }

    let extra_hosts = env("BOOND_ALLOWED_HOSTS").map(|v| v.split(',').map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).collect()).unwrap_or_default();
    let mut config = ServerConfig { data_dir: default_data_dir(), port, extra_hosts, public_url, trust_proxy: flag("BOOND_TRUST_PROXY"), setup_code: env("BOOND_SETUP_CODE") };
    println!("Données : {}", config.data_dir.display());

    // En mode équipe, le premier compte administrateur exige un code : il évite qu'un inconnu
    // qui trouverait l'adresse ne prenne la main avant vous.
    let first = config.setup_code.is_none() && !loopback;
    if first {
        config.setup_code = Some(generate_setup_code());
    }
    let state = build_state(config.clone()).unwrap_or_else(|e| fail(e));
    if state.setup_required().await.unwrap_or(false) {
        if let Some(code) = &config.setup_code {
            println!("\n  Installation : ouvrez l'application et créez le compte administrateur.");
            println!("  Code d'installation : {code}\n");
        }
    }

    let listener = tokio::net::TcpListener::bind((bind.as_str(), port)).await.unwrap_or_else(|e| fail(format!("Impossible d'écouter sur {bind}:{port} : {e}")));
    println!("Boond by XPEHO écoute sur {bind}:{port}{}", config.public_url.as_deref().map(|u| format!(" ({u})")).unwrap_or_default());
    if let Err(e) = serve_until(listener, state, shutdown_signal()).await {
        fail(e);
    }
}
