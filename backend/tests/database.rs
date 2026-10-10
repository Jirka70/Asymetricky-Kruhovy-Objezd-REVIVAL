use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;
use obor_backend::{contract, db, models::*, schema::*};
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

    let app = contract::router(db::pool(&url, 2).unwrap());
    catalog_api(&app, &mut connection).await;

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

// Validate actual HTTP bodies against the YAML, not only serialized DTO fixtures.
async fn contract_response(
    app: &axum::Router,
    uri: &str,
    path: &str,
    status: StatusCode,
) -> serde_json::Value {
    use obor_backend::contract::{self, SPEC};
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), status, "{uri}");
    let media = if status == StatusCode::OK && path != "/skoly/{redizo}" {
        "application/geo+json"
    } else {
        "application/json"
    };
    assert_eq!(response.headers()["content-type"], media);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap()).unwrap();
    let declared = contract::resolve(&SPEC["paths"][path]["get"]["responses"][status.as_str()]);
    let validator = contract::schema_validator(&declared["content"][media]["schema"]);
    let errors: Vec<_> = validator
        .iter_errors(&body)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{uri}: {}", errors.join("; "));
    body
}

async fn catalog_api(app: &axum::Router, connection: &mut diesel::PgConnection) {
    use diesel::connection::SimpleConnection;
    use serde_json::json;
    use std::collections::BTreeSet;
    let schools = contract_response(app, "/api/v1/skoly", "/skoly", StatusCode::OK).await;
    let features = schools["features"].as_array().unwrap();
    assert_eq!(features.len(), 34);
    assert_eq!(
        features
            .iter()
            .filter(|feature| feature["properties"]["pocet_nabidek"] == 0)
            .count(),
        4
    );
    // Form and level filters combine, and totals cover only matching offers.
    for (uri, form, levels, obor) in [
        ("/api/v1/skoly?stupen=H,M", "den", vec!["H", "M"], None),
        (
            "/api/v1/skoly?stupen=H,M&forma=dal",
            "dal",
            vec!["H", "M"],
            None,
        ),
        (
            "/api/v1/skoly?obor=65-51-H%2F01",
            "den",
            vec!["H"],
            Some("65-51-H/01"),
        ),
        (
            "/api/v1/skoly?obor=65-51-H%2F01&stupen=M",
            "den",
            vec!["M"],
            Some("65-51-H/01"),
        ),
    ] {
        let matching: Vec<NabidkaOboru> = nabidka_oboru::table
            .select(NabidkaOboru::as_select())
            .load(connection)
            .unwrap()
            .into_iter()
            .filter(|offer: &NabidkaOboru| {
                offer.forma_studia == form
                    && levels.contains(&&offer.kod_oboru[6..7])
                    && obor.is_none_or(|code| offer.kod_oboru == code)
            })
            .collect();
        let expected: BTreeSet<_> = matching.iter().map(|offer| offer.redizo.as_str()).collect();
        let body = contract_response(app, uri, "/skoly", StatusCode::OK).await;
        let features = body["features"].as_array().unwrap();
        let actual: BTreeSet<_> = features
            .iter()
            .map(|feature| feature["properties"]["redizo"].as_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "{uri}");
        assert_eq!(
            features
                .iter()
                .map(|feature| feature["properties"]["kapacita"].as_i64().unwrap())
                .sum::<i64>(),
            matching
                .iter()
                .map(|offer| i64::from(offer.pocet_prijimanych))
                .sum::<i64>()
        );
        assert_eq!(
            features
                .iter()
                .map(|feature| feature["properties"]["prihlasky"].as_i64().unwrap())
                .sum::<i64>(),
            matching
                .iter()
                .map(|offer| i64::from(offer.loni_pocet_prihlasek))
                .sum::<i64>()
        );
    }
    for feature in features {
        let redizo = feature["properties"]["redizo"].as_str().unwrap();
        let body = contract_response(
            app,
            &format!("/api/v1/skoly/{redizo}"),
            "/skoly/{redizo}",
            StatusCode::OK,
        )
        .await;
        assert_eq!(body["redizo"], redizo);
        assert_eq!(
            feature["geometry"]["coordinates"],
            json!([body["lon"], body["lat"]])
        );
        assert_eq!(body["spadovost"], json!({}));
    }
    contract_response(
        app,
        "/api/v1/skoly/999999999",
        "/skoly/{redizo}",
        StatusCode::NOT_FOUND,
    )
    .await;
    contract_response(
        app,
        "/api/v1/obory/65-51-H%2F01/zamestnavatele",
        "/obory/{kod}/zamestnavatele",
        StatusCode::OK,
    )
    .await;

    // Controlled data tests deduplication, education filtering, mapping semantics,
    // multiple workplaces per company, missing coordinates, and large integer IDs.
    connection.batch_execute(r#"
        INSERT INTO "OBORY" (kod, nazev) VALUES
            ('99-99-H/01', 'Catalog test'), ('99-99-H/02', 'Unmapped test'), ('99-99-H/03', 'No demand test');
        INSERT INTO "PROFESNI_SKUPINY" (cz_isco3, nazev) VALUES
            ('991', 'Test profession A'), ('992', 'Test profession B'), ('993', 'No demand profession');
        INSERT INTO "OBOR_PROFESE" (cz_isco3, kod_oboru, vhodnost) VALUES
            ('991', '99-99-H/01', 1), ('992', '99-99-H/01', 2), ('993', '99-99-H/03', 1);
        INSERT INTO "ZAMESTNAVATELE" (id, ico, nazev, kod_obce, lat, lon) VALUES
            (900000000000000001, '00000001', 'Test company A', '554961', 50.2, 12.8),
            (900000000000000002, '00000001', 'Test company A - missing coordinates', '554961', NULL, NULL),
            (900000000000000003, '00000002', 'Test company B', '554961', 50.3, 12.9);
        INSERT INTO "POPTAVKA_PROFESI" (zamestnavatel_id, cz_isco3, min_vzdelani, pocet_mist, importovano_at) VALUES
            (900000000000000001, '991', 'zakl', 5, '2026-10-10T10:00:00Z'),
            (900000000000000001, '991', 'stredni', 3, '2026-10-10T10:00:00Z'),
            (900000000000000001, '991', 'vyssOdbor', 7, '2026-10-10T10:00:00Z'),
            (900000000000000001, '992', 'stredni', 11, '2026-10-10T10:00:00Z'),
            (900000000000000002, '991', 'stredni', 13, '2026-10-10T10:00:00Z'),
            (900000000000000003, '991', 'vysoka', 17, '2026-10-10T10:00:00Z');
    "#).unwrap();
    let path = "/obory/{kod}/zamestnavatele";
    let uri = "/api/v1/obory/99-99-H%2F01/zamestnavatele";
    let body = contract_response(app, uri, path, StatusCode::OK).await;
    assert_eq!(body["meta"]["existuje"], true);
    assert_eq!(body["meta"]["zamestnavatelu"], 1);
    assert_eq!(body["meta"]["pracovist"], 2);
    assert_eq!(body["meta"]["pocet_mist"], 32);
    assert_eq!(
        body["meta"]["bez_souradnic"],
        json!([{"kod_obce":"554961", "pracovist":1, "pocet_mist":13}])
    );
    let feature = &body["features"][0];
    assert_eq!(body["features"].as_array().unwrap().len(), 1);
    assert_eq!(feature["properties"]["id"], "900000000000000001");
    assert_eq!(feature["properties"]["pocet_mist"], 19);
    assert_eq!(feature["properties"]["profese"][0]["pocet_mist"], 8);
    assert_eq!(feature["geometry"]["coordinates"], json!([12.8, 50.2]));
    assert_eq!(body["meta"]["importovano"], "2026-10-10T10:00:00+00:00");
    let body = contract_response(app, &format!("{uri}?vhodnost=1"), path, StatusCode::OK).await;
    assert_eq!(body["meta"]["pocet_mist"], 21);
    assert_eq!(body["meta"]["skupiny"].as_array().unwrap().len(), 1);
    let body = contract_response(app, &format!("{uri}?jen_ss=false"), path, StatusCode::OK).await;
    assert_eq!(body["meta"]["pocet_mist"], 56);
    assert_eq!(body["meta"]["zamestnavatelu"], 2);
    assert_eq!(body["meta"]["pracovist"], 3);
    assert_eq!(body["features"].as_array().unwrap().len(), 2);
    let body = contract_response(
        app,
        "/api/v1/obory/99-99-H%2F02/zamestnavatele",
        path,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["meta"]["mapovani"], false);
    assert!(body["meta"]["existuje"].is_null());
    let body = contract_response(
        app,
        "/api/v1/obory/99-99-H%2F03/zamestnavatele",
        path,
        StatusCode::OK,
    )
    .await;
    assert_eq!(body["meta"]["mapovani"], true);
    assert_eq!(body["meta"]["existuje"], false);
    assert_eq!(body["meta"]["pocet_mist"], 0);
    contract_response(
        app,
        "/api/v1/obory/99-99-H%2F04/zamestnavatele",
        path,
        StatusCode::NOT_FOUND,
    )
    .await;
    contract_response(
        app,
        &format!("{uri}?vhodnost=3"),
        path,
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;

    // A nullable DB field required by the API must fail visibly, not become 0.
    diesel::update(stredni_skoly::table.find("600008975"))
        .set(stredni_skoly::lat.eq(None::<bigdecimal::BigDecimal>))
        .execute(connection)
        .unwrap();
    contract_response(
        app,
        "/api/v1/skoly",
        "/skoly",
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await;
    contract_response(
        app,
        "/api/v1/skoly/600008975",
        "/skoly/{redizo}",
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await;
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
