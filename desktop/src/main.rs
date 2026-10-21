//! Application de bureau Boond by XPEHO.
//!
//! Une seule fenêtre native affiche l'interface Vue, servie par le serveur Rust embarqué sur
//! 127.0.0.1. Les données (base SQLite, documents) restent sur l'ordinateur de l'utilisateur.
//! Le même serveur répond à l'extension navigateur LinkedIn.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

use boond_http::{DEFAULT_PORT, ServerConfig, build_state, default_data_dir, serve};
use tauri::webview::{DownloadEvent, NewWindowResponse};
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// Premier port libre à partir du port par défaut (l'extension s'attend à `DEFAULT_PORT`).
fn bind_local() -> Result<(TcpListener, u16), String> {
    for port in DEFAULT_PORT..DEFAULT_PORT + 20 {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            return Ok((listener, port));
        }
    }
    Err(format!(
        "Les ports {DEFAULT_PORT} à {} sont occupés. Fermez l'autre instance de Boond by XPEHO puis relancez l'application.",
        DEFAULT_PORT + 19
    ))
}

/// Ouvre une adresse dans le navigateur ou l'application par défaut du système.
fn open_externally(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).spawn();
}

fn downloads_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join("Downloads"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

/// Évite d'écraser un fichier existant : « cv.pdf » devient « cv (1).pdf ».
fn unique_path(dir: &std::path::Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    if !path.exists() {
        return path;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    (1..1000).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap_or(path)
}

fn start() -> Result<String, String> {
    let (listener, port) = bind_local()?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let state = build_state(ServerConfig::local(default_data_dir(), port))?;
    tauri::async_runtime::spawn(async move {
        match tokio::net::TcpListener::from_std(listener) {
            Ok(l) => {
                if let Err(e) = serve(l, state).await {
                    eprintln!("Serveur arrêté : {e}");
                }
            }
            Err(e) => eprintln!("Serveur impossible : {e}"),
        }
    });
    Ok(format!("http://127.0.0.1:{port}"))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let (title, w, h) = ("Boond by XPEHO", 1360.0, 880.0);
            match start() {
                Ok(origin) => {
                    let local = origin.clone();
                    let nav_origin = origin.clone();
                    let download_dir = downloads_dir();
                    WebviewWindowBuilder::new(app, "main", WebviewUrl::External(format!("{origin}/").parse().expect("url")))
                        .title(title)
                        .inner_size(w, h)
                        .min_inner_size(980.0, 640.0)
                        // Tout ce qui n'est pas l'application s'ouvre dans le navigateur ou la messagerie du système.
                        .on_navigation(move |url| {
                            if url.as_str().starts_with(&local) {
                                return true;
                            }
                            if matches!(url.scheme(), "http" | "https" | "mailto" | "tel") {
                                open_externally(url.as_str());
                            }
                            false
                        })
                        .on_new_window(move |url, _| {
                            if !url.as_str().starts_with(&nav_origin) && matches!(url.scheme(), "http" | "https" | "mailto") {
                                open_externally(url.as_str());
                            }
                            NewWindowResponse::Deny
                        })
                        // Les CV et dossiers de compétences se téléchargent dans le dossier Téléchargements.
                        .on_download(move |_, event| {
                            if let DownloadEvent::Requested { destination, .. } = event {
                                let name = destination.file_name().and_then(|n| n.to_str()).unwrap_or("document").to_string();
                                *destination = unique_path(&download_dir, &name);
                            }
                            true
                        })
                        .build()?;
                }
                Err(message) => {
                    eprintln!("{message}");
                    let page = format!("index.html?error={}", urlencoding::encode(&message));
                    WebviewWindowBuilder::new(app, "main", WebviewUrl::App(page.into())).title(title).inner_size(640.0, 420.0).build()?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Boond by XPEHO");
}
