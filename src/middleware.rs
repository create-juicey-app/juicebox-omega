use axum::http::{HeaderValue, header, Request, StatusCode};
use axum::response::Response;
use axum::middleware::Next;
use axum::body::Body;

use crate::config::Config;

// api key validation
pub async fn validate_api_key(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // extract api key hash from request extensions (set during router setup)
    let api_key_hash = req
        .extensions()
        .get::<String>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // get api key from header
    let provided_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("Missing X-API-Key header");
            StatusCode::UNAUTHORIZED
        })?;
    
    // hash the provided key and compare
    let provided_hash = Config::hash_api_key(provided_key);
    
    if provided_hash != *api_key_hash {
        tracing::warn!("🚫 Invalid API key attempt");
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    tracing::debug!("API key validated successfully");
    Ok(next.run(req).await)
}

/// headers & shit
pub async fn add_security_headers(
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:"),
    );
    
    response
}

