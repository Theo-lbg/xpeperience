//! Protections du serveur exposé : hôte, CSRF, code d'installation, cookie sécurisé, limitation des tentatives.
//! Toutes les données sont fictives et vivent dans un dossier temporaire supprimé à la fin.

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Method, Request, Response, StatusCode, header};
use boond_http::{ServerConfig, app, build_state};
use serde_json::{Value, json};
use tower::ServiceExt;

struct Env {
    dir: PathBuf,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn server(public_url: Option<&str>, setup_code: Option<&str>, trust_proxy: bool) -> (axum::Router, Env) {
    let dir = std::env::temp_dir().join(format!("boond-test-{}", uuid::Uuid::new_v4()));
    let mut config = ServerConfig::local(dir.clone(), 47821);
    config.public_url = public_url.map(Into::into);
    config.setup_code = setup_code.map(Into::into);
    config.trust_proxy = trust_proxy;
    (app(build_state(config).unwrap()), Env { dir })
}

async fn send(app: &axum::Router, method: Method, path: &str, host: &str, peer: &str, extra: &[(&str, &str)], body: Option<Value>) -> Response<Body> {
    let mut req = Request::builder().method(method).uri(path).header(header::HOST, host);
    for (k, v) in extra {
        req = req.header(*k, *v);
    }
    let req = match body {
        Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())).unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let mut req = req;
    req.extensions_mut().insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    app.clone().oneshot(req).await.unwrap()
}

async fn json_body(res: Response<Body>) -> Value {
    serde_json::from_slice(&to_bytes(res.into_body(), 1 << 20).await.unwrap()).unwrap_or(Value::Null)
}

const LOCAL: &str = "127.0.0.1:47821";
const PEER: &str = "10.0.0.5:5000";
const WEB: (&str, &str) = ("x-boond-client", "web");

#[tokio::test]
async fn unknown_host_is_rejected_and_public_host_is_accepted() {
    let (app, _e) = server(Some("https://boond.exemple.test"), None, false);
    let r = send(&app, Method::GET, "/api/auth/status", "evil.example", PEER, &[], None).await;
    assert_eq!(r.status(), StatusCode::MISDIRECTED_REQUEST);
    let r = send(&app, Method::GET, "/api/auth/status", "boond.exemple.test", PEER, &[], None).await;
    assert_eq!(r.status(), StatusCode::OK);
    let r = send(&app, Method::GET, "/api/health", LOCAL, "127.0.0.1:1", &[], None).await;
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn writes_without_client_header_and_cross_origin_preflights_are_refused() {
    let (app, _e) = server(None, None, false);
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, PEER, &[], Some(json!({"email":"a@b.test","password":"x"}))).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = send(&app, Method::OPTIONS, "/api/auth/login", LOCAL, PEER, &[("origin", "https://evil.example")], None).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    // Données protégées : pas de session, pas d'accès.
    let r = send(&app, Method::GET, "/api/companies", LOCAL, PEER, &[], None).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let r = send(&app, Method::GET, "/api/ext/status", LOCAL, PEER, &[("authorization", "Bearer bxp_faux")], None).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn setup_requires_the_code_and_cookie_is_secure_over_https() {
    let (app, _e) = server(Some("https://boond.exemple.test"), Some("ABCDE-12345-FGHIJ"), false);
    let host = "boond.exemple.test";
    let st = json_body(send(&app, Method::GET, "/api/auth/status", host, PEER, &[], None).await).await;
    assert_eq!(st["setup_required"], true);
    assert_eq!(st["setup_code_required"], true);

    let account = |code: &str| json!({"email":"admin.fictif@example.test","first_name":"Admin","last_name":"Fictif","password":"motdepasse-test","setup_code":code});
    let r = send(&app, Method::POST, "/api/auth/setup", host, PEER, &[WEB], Some(account("MAUVAIS"))).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(json_body(r).await["fields"][0]["field"], "setup_code");

    let r = send(&app, Method::POST, "/api/auth/setup", host, PEER, &[WEB], Some(account("ABCDE-12345-FGHIJ"))).await;
    assert_eq!(r.status(), StatusCode::OK);
    let cookie = r.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().to_string();
    for part in ["HttpOnly", "SameSite=Strict", "Secure"] {
        assert!(cookie.contains(part), "cookie sans {part} : {cookie}");
    }
    assert!(r.headers().contains_key("strict-transport-security"));

    // Une fois installé, le code n'est plus demandé ni accepté pour créer un second administrateur.
    let st = json_body(send(&app, Method::GET, "/api/auth/status", host, PEER, &[], None).await).await;
    assert_eq!(st["setup_code_required"], false);
    let r = send(&app, Method::POST, "/api/auth/setup", host, PEER, &[WEB], Some(account("ABCDE-12345-FGHIJ"))).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY, "pas de deuxième installation");

    // Avec la session : accès aux données, droits d'administrateur.
    let token = cookie.split(';').next().unwrap().to_string();
    let r = send(&app, Method::GET, "/api/bootstrap", host, PEER, &[("cookie", &token)], None).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(json_body(r).await["address"], "https://boond.exemple.test");
}

#[tokio::test]
async fn repeated_bad_logins_are_throttled_per_account_and_per_address() {
    let (app, _e) = server(None, None, false);
    let r = send(&app, Method::POST, "/api/auth/setup", LOCAL, PEER, &[WEB], Some(json!({"email":"admin.fictif@example.test","first_name":"A","last_name":"B","password":"motdepasse-test"}))).await;
    assert_eq!(r.status(), StatusCode::OK);

    let login = |email: &str, pw: &str| json!({"email": email, "password": pw});
    for _ in 0..6 {
        let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "10.0.0.9:1", &[WEB], Some(login("admin.fictif@example.test", "faux"))).await;
        assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    // Compte bloqué, même avec le bon mot de passe et depuis une autre adresse.
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "10.0.0.77:1", &[WEB], Some(login("admin.fictif@example.test", "motdepasse-test"))).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(r.headers().contains_key(header::RETRY_AFTER));

    // L'adresse est bloquée aussi : un autre compte depuis la même adresse est refusé.
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "10.0.0.9:2", &[WEB], Some(login("autre@example.test", "x"))).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    // Une adresse sans historique n'est pas touchée.
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "10.0.0.88:1", &[WEB], Some(login("autre@example.test", "x"))).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn forwarded_for_is_ignored_unless_the_proxy_is_trusted() {
    // Sans confiance : un en-tête X-Forwarded-For forgé ne permet pas d'échapper au blocage par adresse.
    let (app, _e) = server(None, None, false);
    for i in 0..6 {
        send(&app, Method::POST, "/api/auth/login", LOCAL, "10.0.0.9:1", &[WEB, ("x-forwarded-for", &format!("1.2.3.{i}"))], Some(json!({"email": format!("u{i}@example.test"), "password": "x"}))).await;
    }
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "10.0.0.9:1", &[WEB, ("x-forwarded-for", "8.8.8.8")], Some(json!({"email":"nouveau@example.test","password":"x"}))).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);

    // Avec un proxy de confiance : l'adresse réelle du client est celle du dernier saut ajouté par le proxy.
    let (app, _e) = server(None, None, true);
    for i in 0..6 {
        send(&app, Method::POST, "/api/auth/login", LOCAL, "172.18.0.2:1", &[WEB, ("x-forwarded-for", "203.0.113.7")], Some(json!({"email": format!("u{i}@example.test"), "password": "x"}))).await;
    }
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "172.18.0.2:1", &[WEB, ("x-forwarded-for", "203.0.113.7")], Some(json!({"email":"nouveau@example.test","password":"x"}))).await;
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS, "le client fautif est bloqué");
    let r = send(&app, Method::POST, "/api/auth/login", LOCAL, "172.18.0.2:1", &[WEB, ("x-forwarded-for", "203.0.113.99")], Some(json!({"email":"nouveau@example.test","password":"x"}))).await;
    assert_eq!(r.status(), StatusCode::UNPROCESSABLE_ENTITY, "un autre client derrière le même proxy n'est pas bloqué");
}
