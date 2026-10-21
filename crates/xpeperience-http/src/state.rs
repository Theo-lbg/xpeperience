use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::http::HeaderMap;
use xpeperience_store::Connection;

use crate::ratelimit::Limiter;
use crate::{ApiError, ServerConfig};

/// État partagé : une connexion SQLite protégée par un verrou.
#[derive(Clone)]
pub struct AppState {
    db: Arc<Mutex<Connection>>,
    pub config: Arc<ServerConfig>,
    pub limiter: Arc<Limiter>,
    /// En-têtes `Host` acceptés (adresses locales, adresse publique, hôtes configurés).
    allowed_hosts: Arc<Vec<String>>,
}

fn private_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| format!("Impossible de créer {} : {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

/// `https://xpeperience.exemple.fr:8443/` → `xpeperience.exemple.fr:8443`
fn authority(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty()).then(|| host.to_lowercase())
}

impl AppState {
    pub fn open(config: ServerConfig) -> Result<Self, String> {
        if let Some(url) = &config.public_url
            && !(url.starts_with("https://") || url.starts_with("http://"))
        {
            return Err("XPEPERIENCE_PUBLIC_URL doit commencer par https:// (ou http:// pour un essai local).".into());
        }
        private_dir(&config.data_dir)?;
        private_dir(&config.data_dir.join("documents"))?;
        private_dir(&config.data_dir.join("branding"))?;
        let db_path = xpeperience_store::db_file(&config.data_dir);
        let conn = xpeperience_store::open(&db_path).map_err(|e| format!("Base de données inaccessible : {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600));
        }

        let port = config.port;
        let mut hosts = vec![format!("127.0.0.1:{port}"), format!("localhost:{port}"), format!("[::1]:{port}")];
        if let Some(a) = config.public_url.as_deref().and_then(authority) {
            // Derrière un proxy, l'en-tête Host est celui de l'adresse publique (avec ou sans port par défaut).
            hosts.push(a);
        }
        hosts.extend(config.extra_hosts.iter().map(|h| h.to_lowercase()));

        Ok(Self { db: Arc::new(Mutex::new(conn)), config: Arc::new(config), limiter: Arc::new(Limiter::standard()), allowed_hosts: Arc::new(hosts) })
    }

    pub fn documents_dir(&self) -> PathBuf {
        self.config.data_dir.join("documents")
    }

    pub fn branding_dir(&self) -> PathBuf {
        self.config.data_dir.join("branding")
    }

    /// Le cookie de session est `Secure` (et HSTS envoyé) quand l'adresse publique est en HTTPS.
    pub fn secure(&self) -> bool {
        self.config.public_url.as_deref().is_some_and(|u| u.starts_with("https://"))
    }

    pub fn host_allowed(&self, host: Option<&str>) -> bool {
        if self.allowed_hosts.iter().any(|h| h == "*") {
            return true;
        }
        host.is_some_and(|h| self.allowed_hosts.contains(&h.to_lowercase()))
    }

    /// Adresse du client pour la limitation des tentatives ; `X-Forwarded-For` n'est lu que si le proxy est de confiance.
    pub fn client_ip(&self, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
        if self.config.trust_proxy
            && let Some(ip) = headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit(',').next())
                .and_then(|v| v.trim().parse::<IpAddr>().ok())
        {
            return ip;
        }
        peer.ip()
    }

    /// Exécute un cas d'usage sur la base, hors du fil asynchrone.
    pub async fn run<T, F>(&self, f: F) -> Result<T, ApiError>
    where
        F: FnOnce(&mut Connection) -> xpeperience_domain::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = db.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            f(&mut conn)
        })
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(ApiError::from)
    }

    /// Vrai tant que le premier compte administrateur n'a pas été créé.
    pub async fn setup_required(&self) -> Result<bool, ApiError> {
        self.run(|c| xpeperience_service::auth::setup_required(c)).await
    }
}
