use once_cell::sync::Lazy;
use prometheus::{Encoder, HistogramOpts, HistogramVec, IntCounterVec, Registry, TextEncoder};

pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        prometheus::Opts::new("http_requests_total", "Total number of HTTP requests"),
        &["method", "path", "status"],
    )
    .expect("create http_requests_total")
});

pub static HTTP_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        ),
        &["method", "path"],
    )
    .expect("create http_request_duration_seconds")
});

pub fn init_metrics() {
    let _ = REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(HTTP_REQUEST_DURATION_SECONDS.clone()));
}

pub fn encode_metrics() -> String {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let _ = encoder.encode(&metric_families, &mut buffer);
    String::from_utf8(buffer).unwrap_or_default()
}
