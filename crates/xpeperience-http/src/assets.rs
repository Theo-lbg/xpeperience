//! Fichiers de l'interface Vue, intégrés au binaire en version publiée.

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

use crate::error::ApiError;

#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct Web;

fn file(path: &str) -> Option<Response> {
    let asset = Web::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut res = (StatusCode::OK, asset.data.into_owned()).into_response();
    res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).ok()?);
    let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
    res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    Some(res)
}

/// Sert un fichier de l'interface, ou `index.html` pour les routes de l'application.
pub async fn spa(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        return ApiError::not_found().into_response();
    }
    if !path.is_empty()
        && let Some(res) = file(path)
    {
        return res;
    }
    file("index.html").unwrap_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, "Interface non construite : lancez `npm run build` dans web/.").into_response()
    })
}
