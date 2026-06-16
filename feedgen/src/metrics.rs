use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use prometheus::{
    gather, register_counter_vec, register_histogram_vec, CounterVec, HistogramVec, TextEncoder,
};
use std::sync::LazyLock;

/// Total HTTP requests counter by method, path, and status code.
pub static HTTP_REQUESTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "feedgen_http_requests_total",
        "Total number of HTTP requests made to the feed generator",
        &["method", "path", "status"]
    )
    .expect("Failed to register feedgen_http_requests_total")
});

/// HTTP request duration histogram by method and path.
pub static HTTP_REQUEST_DURATION_SECONDS: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "feedgen_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"]
    )
    .expect("Failed to register feedgen_http_request_duration_seconds")
});

/// Database query counter by query type (e.g., "select", "insert", "delete").
pub static DB_QUERIES_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "feedgen_db_queries_total",
        "Total number of database queries executed",
        &["query_type"]
    )
    .expect("Failed to register feedgen_db_queries_total")
});

/// AXUM middleware that records request count and duration for every incoming request.
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[&method, &path])
        .observe(duration);

    response
}

/// Handler for the `/metrics` endpoint. Exposes all registered Prometheus metrics.
pub async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = gather();
    let mut buffer = String::new();
    if let Err(e) = encoder.encode_utf8(&metric_families, &mut buffer) {
        eprintln!("Failed to encode Prometheus metrics: {}", e);
        return String::new();
    }
    buffer
}
