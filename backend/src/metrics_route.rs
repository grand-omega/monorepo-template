use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue};
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;

#[derive(Clone)]
pub struct MetricsState {
    pub handle: Arc<PrometheusHandle>,
}

pub async fn render(State(state): State<MetricsState>) -> (HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    (headers, state.handle.render())
}
