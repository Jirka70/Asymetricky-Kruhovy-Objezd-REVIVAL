use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use obor_backend::contract;
use std::{
    collections::BTreeSet,
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceExt;

#[derive(Clone)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn field<'a>(line: &'a str, name: &str) -> &'a str {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{name}=")))
        .unwrap_or_else(|| panic!("Missing {name}: {line}"))
}

#[test]
fn logs_request_and_response_with_correlated_id_status_and_elapsed_time() {
    let output = LogBuffer(Arc::new(Mutex::new(Vec::new())));
    let writer = output.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("obor_backend=info")
        .with_ansi(false)
        .without_time()
        .with_writer(move || writer.clone())
        .finish();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let cases = [
        (Method::GET, "/docs?token=private", StatusCode::OK),
        (Method::GET, "/missing", StatusCode::NOT_FOUND),
        (Method::POST, "/docs", StatusCode::METHOD_NOT_ALLOWED),
        (
            Method::GET,
            "/api/v1/zsj?uroven=invalid",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            Method::GET,
            "/api/v1/zsj/seznam",
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ];
    tracing::subscriber::with_default(subscriber, || {
        runtime.block_on(async {
            let pool = diesel::r2d2::Pool::builder()
                .max_size(1)
                .connection_timeout(Duration::from_millis(50))
                .build_unchecked(
                    diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(
                        "postgres://localhost:1/unused_logging_test",
                    ),
                );
            let app = contract::router(pool);
            for (method, uri, status) in &cases {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method(method.clone())
                            .uri(*uri)
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), *status);
            }
        });
    });
    let logs = String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
    assert!(!logs.contains("private"));
    let events: Vec<_> = logs
        .lines()
        .filter(|line| line.contains("Request received") || line.contains("Response ready"))
        .collect();
    assert_eq!(events.len(), cases.len() * 2);
    let mut ids = BTreeSet::new();
    for (pair, (method, uri, status)) in events.chunks_exact(2).zip(cases) {
        assert!(pair[0].contains("Request received"));
        assert!(pair[1].contains("Response ready"));
        let id = field(pair[0], "request_id");
        assert!(ids.insert(id));
        assert_eq!(field(pair[1], "request_id"), id);
        for line in pair {
            assert_eq!(field(line, "method"), method.as_str());
            assert_eq!(field(line, "path"), uri.split('?').next().unwrap());
        }
        assert_eq!(field(pair[1], "status"), status.as_u16().to_string());
        let elapsed_ms: f64 = field(pair[1], "elapsed_ms").parse().unwrap();
        assert!(elapsed_ms.is_finite() && elapsed_ms >= 0.0);
        if status == StatusCode::SERVICE_UNAVAILABLE {
            assert!(
                elapsed_ms >= 40.0,
                "Timing must include waiting for the database pool"
            );
        }
    }
}
