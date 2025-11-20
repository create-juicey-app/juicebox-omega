use juicebox_omega::middleware::{add_security_headers, validate_api_key};
use juicebox_omega::config::Config;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn;
use axum::routing::get;
use axum::Router;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_add_security_headers() {
    let app = Router::new()
        .route("/", get(|| async { "hello" }))
        .layer(from_fn(add_security_headers));

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let headers = response.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
}

#[tokio::test]
async fn test_validate_api_key() {
    let correct_key = "secret";
    let correct_hash = Config::hash_api_key(correct_key);

    let app = Router::new()
        .route("/", get(|| async { "hello" }))
        .layer(from_fn(validate_api_key))
        .layer(axum::Extension(correct_hash));

    // Test missing header
    let response = app.clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test wrong key
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-API-Key", "wrong")
                .body(Body::empty())
                .unwrap()
            )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test correct key
    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-API-Key", correct_key)
                .body(Body::empty())
                .unwrap()
            )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
