//! Serveur HTTP de XpePerience : API REST, sessions et fichiers de l'interface.
//!
//! Le même serveur est utilisé par l'application de bureau (Tauri) et par le
//! binaire autonome `xpeperience-server` (mode équipe, derrière un proxy HTTPS).

mod assets;
mod error;
mod extract;
mod guard;
mod ratelimit;
mod routes;
mod state;

use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;

pub use error::ApiError;
pub use state::AppState;

/// Port local par défaut : l'extension LinkedIn s'y connecte.
pub const DEFAULT_PORT: u16 = 47821;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Dossier des données (base SQLite, documents, logo).
    pub data_dir: PathBuf,
    /// Port réellement écouté, utilisé pour la vérification de l'en-tête `Host`.
    pub port: u16,
    /// Hôtes acceptés en plus de `127.0.0.1` et `localhost` ; `*` désactive la vérification.
    pub extra_hosts: Vec<String>,
    /// Adresse publique (ex. `https://xpeperience.exemple.fr`) quand le serveur est derrière un proxy.
    /// Si elle commence par `https://`, le cookie de session est `Secure` et HSTS est envoyé.
    pub public_url: Option<String>,
    /// Fait confiance à `X-Forwarded-For` (uniquement derrière un proxy de confiance).
    pub trust_proxy: bool,
    /// Code requis pour créer le premier compte administrateur (mode équipe).
    pub setup_code: Option<String>,
}

impl ServerConfig {
    /// Configuration de l'application de bureau : local uniquement, sans proxy ni code d'installation.
    pub fn local(data_dir: PathBuf, port: u16) -> Self {
        Self { data_dir, port, extra_hosts: Vec::new(), public_url: None, trust_proxy: false, setup_code: None }
    }
}

/// Dossier de données de l'utilisateur, surchargeable par `XPEPERIENCE_DATA_DIR`.
pub fn default_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XPEPERIENCE_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let current = base.join("fr.xpeho.xpeperience");
    // Avant le renommage du produit, le dossier s'appelait `fr.xpeho.boond` : on le réutilise tel quel
    // pour ne perdre aucune donnée.
    let legacy = base.join("fr.xpeho.boond");
    if !current.exists() && legacy.is_dir() { legacy } else { current }
}

/// Ouvre la base et prépare l'état partagé du serveur.
pub fn build_state(config: ServerConfig) -> Result<AppState, String> {
    AppState::open(config)
}

/// Construit le routeur de l'application (utilisé aussi par les tests).
pub fn app(state: AppState) -> axum::Router {
    routes::router(state)
}

/// Code d'installation aléatoire (64 bits d'entropie), à usage unique.
pub fn generate_setup_code() -> String {
    let raw = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
    format!("{}-{}-{}", &raw[0..5], &raw[5..10], &raw[10..15])
}

/// Sert l'application sur un port déjà ouvert, jusqu'à l'arrêt du processus.
pub async fn serve(listener: tokio::net::TcpListener, state: AppState) -> std::io::Result<()> {
    serve_until(listener, state, std::future::pending()).await
}

/// Sert l'application jusqu'à ce que `shutdown` se termine : les requêtes en cours sont terminées proprement.
pub async fn serve_until(
    listener: tokio::net::TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, app(state).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown)
        .await
}
