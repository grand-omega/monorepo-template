use crate::AppState;
use crate::config::AppEnv;
use crate::health;
use crate::middleware::http_metrics;
use crate::middleware::request_id::UuidV7RequestId;
use crate::middleware::security_headers;
use crate::openapi::ApiDoc;
use crate::{admin, auth, users};
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::routing::get;
use http::StatusCode;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
const X_CSRF_TOKEN: HeaderName = HeaderName::from_static("x-csrf-token");

pub fn build_router(state: AppState) -> Router {
    let cors = build_cors(&state);
    let body_limit = 1024usize * 1024; // 1 MiB

    let app_routes = Router::new()
        .nest("/auth", auth::routes::router(state.clone()))
        .merge(users::routes::router(state.clone()));

    let openapi_doc = ApiDoc::openapi();
    let mut router: Router<AppState> = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/admin/api", admin::routes::router(state.clone()))
        .nest("/v1", app_routes)
        .layer(DefaultBodyLimit::max(body_limit));

    // OpenAPI JSON is published in all environments — the SPA codegens against
    // it; the schema isn't sensitive. In dev we additionally mount Swagger UI
    // which carries its own JSON route, so we let SwaggerUi own /openapi.json
    // there to avoid a duplicate registration.
    if state.config.env.is_dev() {
        let swagger: Router<AppState> = SwaggerUi::new("/docs")
            .url("/openapi.json", openapi_doc)
            .into();
        router = router.merge(swagger);
    } else {
        router = router.route(
            "/openapi.json",
            get({
                let doc = openapi_doc.clone();
                move || async move { axum::Json(doc) }
            }),
        );
    }

    let middleware = ServiceBuilder::new()
        .layer(CatchPanicLayer::new())
        .layer(SetSensitiveRequestHeadersLayer::new(std::iter::once(
            header::AUTHORIZATION,
        )))
        .layer(SetRequestIdLayer::new(X_REQUEST_ID, UuidV7RequestId))
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(axum::middleware::from_fn(http_metrics::record))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ));

    let router = router.layer(middleware);
    let router = security_headers::apply(router);

    router.with_state(state)
}

fn build_cors(state: &AppState) -> CorsLayer {
    if state.config.cors_allowed_origins.is_empty() {
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                X_REQUEST_ID,
                X_CSRF_TOKEN,
            ])
    } else {
        let origins: Vec<HeaderValue> = state
            .config
            .cors_allowed_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                X_REQUEST_ID,
                X_CSRF_TOKEN,
            ])
            .allow_credentials(state.config.env == AppEnv::Prod)
    }
}
