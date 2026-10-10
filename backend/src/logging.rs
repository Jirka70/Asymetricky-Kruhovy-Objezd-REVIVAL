//! Request lifecycle logs for all HTTP routes.
use axum::{extract::Request, middleware::Next, response::Response};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) async fn log_request(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    tracing::info!(request_id, %method, %path, "Request received");

    let response = next.run(request).await;
    tracing::info!(
        request_id,
        %method,
        %path,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
        "Response ready"
    );
    response
}
