use axum::Router;
use http::HeaderName;
use http::HeaderValue;
use tower_http::set_header::SetResponseHeaderLayer;

pub fn apply<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .layer(static_header("x-content-type-options", "nosniff"))
        .layer(static_header("x-frame-options", "DENY"))
        .layer(static_header("referrer-policy", "no-referrer"))
        .layer(static_header(
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ))
        .layer(static_header(
            "permissions-policy",
            "geolocation=(), microphone=(), camera=()",
        ))
        .layer(static_header("cross-origin-opener-policy", "same-origin"))
        .layer(static_header("cross-origin-resource-policy", "same-origin"))
}

fn static_header(name: &'static str, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static(name),
        HeaderValue::from_static(value),
    )
}
