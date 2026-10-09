use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;
use obor_backend::{api, db, models::*, schema::*};
use tower::ServiceExt;

// Always use a fresh owned container: rollback must never touch a user database.
#[tokio::test]
async fn migrations_and_api() {
    let database = TestDatabase::start();
    let url = database.url();
    let mut connection = db::connect(&url).unwrap();
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }
    let existing = diesel::sql_query("SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tablename NOT IN ('spatial_ref_sys')")
        .get_result::<Count>(&mut connection).unwrap();
    assert_eq!(
        existing.count, 0,
        "Test refused: database must have no application tables"
    );
    assert_eq!(db::migrate(&mut connection).unwrap(), 7);
    assert_eq!(db::migrate(&mut connection).unwrap(), 0);
    assert_eq!(
        zsj::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        839
    );
    assert_eq!(
        stredni_skoly::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        34
    );
    assert_eq!(
        obory::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        81
    );
    assert_eq!(
        nabidka_oboru::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        140
    );
    assert_eq!(
        zamestnavatele::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        422
    );
    assert_eq!(
        profesni_skupiny::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        88
    );
    assert_eq!(
        poptavka_profesi::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        641
    );
    assert_eq!(
        obor_profese::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        291
    );
    assert_eq!(
        poptavka_profesi::table
            .select(diesel::dsl::sum(poptavka_profesi::pocet_mist))
            .first::<Option<i64>>(&mut connection)
            .unwrap(),
        Some(1889)
    );
    let school = stredni_skoly::table
        .find("600008975")
        .select(Skola::as_select())
        .first(&mut connection)
        .unwrap();
    assert_eq!(school.kod_zsj.as_deref(), Some("000540"));
    assert!(school.lat.is_some());

    let app = api::router(db::pool(&url, 2).unwrap());
    for (endpoint, expected) in [
        ("skoly", 34),
        ("zsj", 839),
        ("obory", 81),
        ("nabidky", 140),
        ("zamestnavatele", 422),
        ("profesni-skupiny", 88),
        ("poptavka-profesi", 641),
        ("obor-profese", 291),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/{endpoint}?limit=1000"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{endpoint}");
        let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows.len(), expected, "{endpoint}");
    }
    for (uri, expected) in [
        ("/health", StatusCode::OK),
        ("/api/skoly/600008975", StatusCode::OK),
        ("/api/zsj/000540", StatusCode::OK),
        ("/api/skoly/does-not-exist", StatusCode::NOT_FOUND),
        ("/api/zsj/does-not-exist", StatusCode::NOT_FOUND),
        ("/api/zsj?limit=0", StatusCode::BAD_REQUEST),
        ("/api/skoly?offset=-1", StatusCode::BAD_REQUEST),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{uri}");
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/zsj?limit=2&offset=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(response.into_body(), 100_000).await.unwrap();
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["kod"], "000027");

    assert_eq!(
        connection
            .revert_all_migrations(db::MIGRATIONS)
            .unwrap()
            .len(),
        7
    );
    assert_eq!(db::migrate(&mut connection).unwrap(), 7);
    assert_eq!(
        zsj::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        839
    );
    assert_eq!(
        poptavka_profesi::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        641
    );
}

/// Owns one disposable PostGIS container, including cleanup when assertions panic.
struct TestDatabase {
    name: String,
    port: u16,
}
impl TestDatabase {
    fn start() -> Self {
        use std::process::Command;
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut database = Self {
            name: format!("obor-backend-test-{}-{suffix}", std::process::id()),
            port: 0,
        };
        let output = Command::new("docker").args([
            "run", "--rm", "--detach", "--name", &database.name,
            "--platform", "linux/amd64",
            "--env", "POSTGRES_PASSWORD=disposable-test-only",
            "--env", "POSTGRES_DB=backend_test",
            "--publish", "127.0.0.1::5432",
            "--tmpfs", "/var/lib/postgresql",
            "postgis/postgis:18-3.6",
        ]).output().expect("Database tests require Docker; start Docker Desktop and rerun cargo test. OpenAPI-only tests: cargo test --test openapi_contract");
        assert!(
            output.status.success(),
            "Could not start disposable PostGIS: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = Command::new("docker")
            .args(["port", &database.name, "5432/tcp"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Could not inspect test database port"
        );
        database.port = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .rsplit(':')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        for _ in 0..60 {
            let ready = Command::new("docker")
                .args([
                    "exec",
                    &database.name,
                    "pg_isready",
                    "-h",
                    "127.0.0.1",
                    "-U",
                    "postgres",
                    "-d",
                    "backend_test",
                ])
                .output()
                .is_ok_and(|output| output.status.success());
            if ready {
                return database;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        panic!("Disposable PostGIS database did not become ready within 30 seconds");
    }
    fn url(&self) -> String {
        format!(
            "postgres://postgres:disposable-test-only@127.0.0.1:{}/backend_test",
            self.port
        )
    }
}
impl Drop for TestDatabase {
    fn drop(&mut self) {
        match std::process::Command::new("docker")
            .args(["rm", "--force", &self.name])
            .output()
        {
            Ok(output) if output.status.success() => {}
            Ok(output) => eprintln!(
                "Could not clean up test container {}: {}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) => eprintln!("Could not clean up test container {}: {error}", self.name),
        }
    }
}
