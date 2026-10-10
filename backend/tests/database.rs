mod support;
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
    // Exercise upgrading a database with the original seven migrations applied.
    let migrations = connection.pending_migrations(db::MIGRATIONS).unwrap();
    for migration in &migrations[..7] {
        connection.run_migration(migration.as_ref()).unwrap();
    }
    assert_eq!(db::migrate(&mut connection).unwrap(), 3);
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

    analytical_data(&mut connection);

    let app = contract::router(db::pool(&url, 2).unwrap());
    student_school_data(&app, &mut connection, db::pool(&url, 2).unwrap()).await;
    zsj_api(&app, &mut connection).await;
    zsj_accessibility_api(&app, &mut connection).await;
    simulation_api(&app, &mut connection).await;
    batch_simulation_api(&app, &mut connection).await;
    catalog_api(&app, &mut connection).await;
    program_detail_api(&app, &mut connection).await;

    assert_eq!(
        connection
            .revert_all_migrations(db::MIGRATIONS)
            .unwrap()
            .len(),
        10
    );
    let remaining = diesel::sql_query("SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tablename IN ('ZSJ', 'DEMO_SKUPINA', 'DATA_DEMOGRAFIE_ZSJ', 'DOJEZDOVE_DOBY', 'SIMULATION_OBCE')")
        .get_result::<Count>(&mut connection).unwrap();
    assert_eq!(remaining.count, 0);
    assert_eq!(db::migrate(&mut connection).unwrap(), 10);
    analytical_data(&mut connection);
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

// Check real seeded data, typed joins, identifier normalization, and constraints.
fn analytical_data(connection: &mut PgConnection) {
    assert_eq!(
        demo_skupina::table
            .count()
            .get_result::<i64>(connection)
            .unwrap(),
        21
    );
    assert_eq!(
        data_demografie_zsj::table
            .count()
            .get_result::<i64>(connection)
            .unwrap(),
        17_619
    );
    assert_eq!(
        data_demografie_zsj::table
            .select(diesel::dsl::sum(data_demografie_zsj::populace))
            .first::<Option<i64>>(connection)
            .unwrap(),
        Some(279_103)
    );
    assert_eq!(
        zsj::table
            .filter(zsj::kod_obce.is_not_null())
            .count()
            .get_result::<i64>(connection)
            .unwrap(),
        839
    );
    assert_eq!(
        zsj::table
            .select(zsj::kod_obce)
            .distinct()
            .load::<Option<String>>(connection)
            .unwrap()
            .len(),
        134
    );
    let children = data_demografie_zsj::table
        .inner_join(demo_skupina::table)
        .inner_join(zsj::table)
        .filter(demo_skupina::vek_od_do.eq("10-14"))
        .filter(data_demografie_zsj::rok.eq(2021))
        .select(DemografieZsj::as_select())
        .load::<DemografieZsj>(connection)
        .unwrap();
    assert_eq!(children.len(), 839);
    assert!(
        children
            .iter()
            .map(|row| i64::from(row.populace))
            .sum::<i64>()
            > 0
    );
    let group = demo_skupina::table
        .find("1300100014")
        .select(DemoSkupina::as_select())
        .first::<DemoSkupina>(connection)
        .unwrap();
    assert_eq!(group.vek_od_do, "10-14");
    assert_eq!(
        dojezdove_doby::table
            .count()
            .get_result::<i64>(connection)
            .unwrap(),
        28_526
    );
    assert_eq!(
        dojezdove_doby::table
            .inner_join(zsj::table)
            .inner_join(stredni_skoly::table)
            .count()
            .get_result::<i64>(connection)
            .unwrap(),
        28_526
    );
    assert_eq!(
        dojezdove_doby::table
            .select(dojezdove_doby::slot_prijezdu)
            .distinct()
            .load::<String>(connection)
            .unwrap(),
        vec!["07:00-08:00"]
    );
    let time = dojezdove_doby::table
        .find(("000019", "600022854", "07:00-08:00"))
        .select(DojezdovaDoba::as_select())
        .first::<DojezdovaDoba>(connection)
        .unwrap();
    assert!((time.doba_jizdy.unwrap() - 73.48).abs() < 0.001);
    let no_route = dojezdove_doby::table
        .find(("000019", "600008975", "07:00-08:00"))
        .select(DojezdovaDoba::as_select())
        .first::<DojezdovaDoba>(connection)
        .unwrap();
    assert!(no_route.doba_jizdy.is_none());
    // With unique keys and valid references, 839 * 34 rows prove full coverage.
    for invalid in ["-1", "'Infinity'::real", "'NaN'::real"] {
        let result = diesel::sql_query(format!(
            "INSERT INTO public.\"DOJEZDOVE_DOBY\" VALUES ('000019', '600022854', 'invalid-test', {invalid})"
        )).execute(connection);
        assert!(matches!(
            result,
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::CheckViolation,
                _
            ))
        ));
    }
    let result = diesel::sql_query(
        "INSERT INTO public.\"DOJEZDOVE_DOBY\" VALUES ('19', '600022854', 'invalid-test', 1)",
    )
    .execute(connection);
    assert!(matches!(
        result,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::ForeignKeyViolation,
            _
        ))
    ));
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
    let media = if status == StatusCode::OK
        && (matches!(
            path,
            "/skoly" | "/zsj" | "/student/trasa" | "/obory/{kod}/zamestnavatele"
        ) || (path == "/simulace" && uri.contains("format=geojson")))
    {
        "application/geo+json"
    } else {
        "application/json"
    };
    assert_eq!(response.headers()["content-type"], media);
    let body_limit = if matches!(path, "/zsj/seznam" | "/zsj" | "/simulace") {
        64_000_000
    } else {
        2_000_000
    };
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), body_limit).await.unwrap()).unwrap();
    let declared = contract::resolve(&SPEC["paths"][path]["get"]["responses"][status.as_str()]);
    let validator = contract::schema_validator(&declared["content"][media]["schema"]);
    let errors: Vec<_> = validator
        .iter_errors(&body)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{uri}: {}", errors.join("; "));
    body
}

async fn student_school_data(_app: &axum::Router, connection: &mut PgConnection, pool: db::DbPool) {
    use bigdecimal::ToPrimitive;
    use diesel::connection::SimpleConnection;
    use obor_backend::requests::{Forma, KodOboru, Scenar, StudentSkolyQuery};
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};
    let origin = zsj::table
        .find("000019")
        .select(Zsj::as_select())
        .first::<Zsj>(connection)
        .unwrap();
    let params = StudentSkolyQuery {
        lat: origin.lat,
        lon: origin.lon,
        obor: None,
        forma: Forma::Den,
        max_min: 60,
        scenar: Scenar::Rano,
    };
    let inputs = obor_backend::student::load_school_data(pool.clone(), params.clone())
        .await
        .unwrap();
    assert_eq!(inputs.origin.as_ref().unwrap().kod, "000019");
    assert_eq!(inputs.schools.len(), 34);
    let expected = nabidka_oboru::table
        .filter(nabidka_oboru::forma_studia.eq("den"))
        .select(nabidka_oboru::id)
        .load::<uuid::Uuid>(connection)
        .unwrap()
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        inputs
            .offerings
            .iter()
            .map(|(o, _)| o.id)
            .collect::<BTreeSet<_>>(),
        expected
    );
    for (form, raw) in [(Forma::Den, "den"), (Forma::Dal, "dal")] {
        let filtered = obor_backend::student::load_school_data(
            pool.clone(),
            StudentSkolyQuery {
                forma: form,
                obor: Some(KodOboru("65-51-H/01".into())),
                ..params.clone()
            },
        )
        .await
        .unwrap();
        let expected = nabidka_oboru::table
            .filter(nabidka_oboru::kod_oboru.eq("65-51-H/01"))
            .filter(nabidka_oboru::forma_studia.eq(raw))
            .select(nabidka_oboru::redizo)
            .load::<String>(connection)
            .unwrap()
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            filtered
                .schools
                .iter()
                .map(|s| s.redizo.clone())
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert!(
            filtered
                .offerings
                .iter()
                .all(|(o, p)| o.forma_studia == raw && o.kod_oboru == p.kod)
        );
    }
    #[derive(QueryableByName)]
    struct BoundaryPoint {
        #[diesel(sql_type=diesel::sql_types::Double)]
        lat: f64,
        #[diesel(sql_type=diesel::sql_types::Double)]
        lon: f64,
    }
    let point=diesel::sql_query(r#"SELECT ST_Y(ST_StartPoint(ST_ExteriorRing(boundary))) AS lat, ST_X(ST_StartPoint(ST_ExteriorRing(boundary))) AS lon FROM "ZSJ" WHERE kod='000019'"#).get_result::<BoundaryPoint>(connection).unwrap();
    assert_eq!(
        obor_backend::student::load_school_data(
            pool.clone(),
            StudentSkolyQuery {
                lat: point.lat,
                lon: point.lon,
                ..params.clone()
            }
        )
        .await
        .unwrap()
        .origin
        .unwrap()
        .kod,
        "000019"
    );
    let durations = BTreeMap::from([
        ("600022854", Some(60.0)),
        ("600008975", Some(60.4)),
        ("600009301", Some(60.1)),
        ("600019632", None),
        ("600009009", Some(60.0)),
    ]);
    let targets = inputs
        .schools
        .iter()
        .map(|s| {
            let lat = s.lat.as_ref().unwrap().to_f64().unwrap();
            let lon = s.lon.as_ref().unwrap().to_f64().unwrap();
            (
                (lat.to_bits(), lon.to_bits()),
                durations.get(s.redizo.as_str()).copied().flatten(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mock = support::MockOtp::start(move |body| {
        let destination = support::destination(body);
        let duration = targets[&(destination.0.to_bits(), destination.1.to_bits())];
        let candidates = duration
            .map(|m| {
                vec![support::itinerary(
                    m,
                    "2026-10-12T07:50:00+02:00",
                    support::origin(body),
                    destination,
                )]
            })
            .unwrap_or_default();
        (StatusCode::OK, support::response(candidates))
    })
    .await;
    let app = contract::router_with_otp(pool.clone(), mock.client.clone());
    connection.batch_execute(r#"
        INSERT INTO "OBORY" (kod,nazev) VALUES ('99-98-H/01','Student response test');
        INSERT INTO "NABIDKA_OBORU" (id,redizo,kod_oboru,forma_studia,delka_studia,pocet_prijimanych,loni_pocet_prihlasek) VALUES
        ('98000000-0000-0000-0000-000000000001','600022854','99-98-H/01','den',3,5,50),
        ('98000000-0000-0000-0000-000000000002','600022854','99-98-H/01','den',3,0,1),
        ('98000000-0000-0000-0000-000000000003','600008975','99-98-H/01','den',3,10,40),
        ('98000000-0000-0000-0000-000000000004','600009301','99-98-H/01','den',3,10,40),
        ('98000000-0000-0000-0000-000000000005','600019632','99-98-H/01','den',3,10,40),
        ('98000000-0000-0000-0000-000000000006','600009009','99-98-H/01','den',3,10,40),
        ('98000000-0000-0000-0000-000000000007','600008975','99-98-H/01','dal',3,3,9);
        UPDATE stredni_skoly SET nazev='Equal times' WHERE redizo IN ('600022854','600009009');
        ALTER TABLE "DOJEZDOVE_DOBY" RENAME TO "TEST_DOJEZDOVE_DOBY";
    "#).unwrap();
    let base = format!(
        "/api/v1/student/skoly?lat={}&lon={}&max_min=60",
        origin.lat, origin.lon
    );
    let body = contract_response(
        &app,
        &format!("{base}&obor=99-98-H%2F01"),
        "/student/skoly",
        StatusCode::OK,
    )
    .await;
    let rows = body["data"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| r["redizo"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "600009009",
            "600022854",
            "600009301",
            "600008975",
            "600019632"
        ]
    );
    assert_eq!(body["meta"]["v_dosahu"], 2);
    assert_eq!(body["meta"]["mimo_dosah"], 3);
    assert_eq!(body["meta"]["den"], "2026-10-12");
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row["v_dosahu"], i < 2);
        if i < 4 {
            assert_eq!(row["spoj"]["cas_min"], 60);
            assert_eq!(row["spoj"]["chuze_m"], 124);
            assert_eq!(row["spoj"]["prestupy"], 0);
            assert_eq!(row["spoj"]["prijezd"], "07:50");
        }
    }
    assert_eq!(rows[4]["spoj"]["stav"], "bez_spojeni");
    assert!(rows[4]["spoj"].get("cas_min").is_none());
    assert_eq!(rows[1]["nabidky"].as_array().unwrap().len(), 2);
    assert_eq!(rows[1]["nabidky"][0]["prihlasky_na_misto"], 10.0);
    assert!(rows[1]["nabidky"][1].get("prihlasky_na_misto").is_none());
    let unfiltered = contract_response(&app, &base, "/student/skoly", StatusCode::OK).await;
    assert_eq!(unfiltered["data"].as_array().unwrap().len(), 2);
    assert_eq!(unfiltered["meta"]["mimo_dosah"], 32);
    let dal = contract_response(
        &app,
        &format!("{base}&obor=99-98-H%2F01&forma=dal"),
        "/student/skoly",
        StatusCode::OK,
    )
    .await;
    assert_eq!(dal["data"].as_array().unwrap().len(), 1);
    assert_eq!(dal["data"][0]["nabidky"][0]["forma"], "dal");
    assert_eq!(dal["data"][0]["v_dosahu"], false);
    let outside = contract_response(
        &app,
        "/api/v1/student/skoly?lat=49.9&lon=12.0",
        "/student/skoly",
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    assert_eq!(outside["error"]["kod"], "mimo_uzemi");
    let empty = contract_response(
        &app,
        &format!("{base}&obor=99-99-H%2F99"),
        "/student/skoly",
        StatusCode::OK,
    )
    .await;
    assert_eq!(empty["data"], json!([]));
    assert_eq!(empty["meta"]["v_dosahu"], 0);
    let count = mock.requests.lock().unwrap().len();
    let route_uri = format!(
        "/api/v1/student/trasa?lat={}&lon={}&redizo=600022854",
        origin.lat, origin.lon
    );
    let route = contract_response(&app, &route_uri, "/student/trasa", StatusCode::OK).await;
    assert_eq!(route["spoje"][0]["cas_min"], 60);
    assert_eq!(route["features"][0]["properties"]["druh"], "WALK");
    assert_eq!(route["features"][0]["geometry"]["type"], "LineString");
    assert_eq!(
        mock.requests.lock().unwrap().len(),
        count,
        "Student routes reuse school-search cache"
    );
    let no_route = contract_response(
        &app,
        &route_uri.replace("600022854", "600019632"),
        "/student/trasa",
        StatusCode::OK,
    )
    .await;
    assert_eq!(no_route["spoje"], json!([]));
    assert_eq!(no_route["features"], json!([]));
    contract_response(
        &app,
        &route_uri.replace("600022854", "600000000"),
        "/student/trasa",
        StatusCode::NOT_FOUND,
    )
    .await;
    contract_response(
        &app,
        "/api/v1/student/trasa?lat=49.9&lon=12.0&redizo=600022854",
        "/student/trasa",
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    let failed =
        support::MockOtp::start(|_| (StatusCode::INTERNAL_SERVER_ERROR, json!({"error":"test"})))
            .await;
    let unavailable = contract::router_with_otp(pool.clone(), failed.client.clone());
    contract_response(
        &unavailable,
        &format!("{base}&obor=99-98-H%2F01"),
        "/student/skoly",
        StatusCode::BAD_GATEWAY,
    )
    .await;
    contract_response(
        &unavailable,
        &route_uri,
        "/student/trasa",
        StatusCode::BAD_GATEWAY,
    )
    .await;
    // Validate database failures before contacting OTP.
    diesel::update(stredni_skoly::table.find("600022854"))
        .set(stredni_skoly::nazev.eq(None::<String>))
        .execute(connection)
        .unwrap();
    contract_response(
        &app,
        &format!("{base}&obor=99-98-H%2F01"),
        "/student/skoly",
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await;
    for school in &inputs.schools {
        diesel::update(stredni_skoly::table.find(&school.redizo))
            .set(stredni_skoly::nazev.eq(school.nazev.clone()))
            .execute(connection)
            .unwrap();
    }
    connection
        .batch_execute(r#"ALTER TABLE "TEST_DOJEZDOVE_DOBY" RENAME TO "DOJEZDOVE_DOBY";"#)
        .unwrap();
    diesel::delete(nabidka_oboru::table.filter(nabidka_oboru::kod_oboru.eq("99-98-H/01")))
        .execute(connection)
        .unwrap();
    diesel::delete(obory::table.find("99-98-H/01"))
        .execute(connection)
        .unwrap();
}

async fn zsj_api(app: &axum::Router, connection: &mut PgConnection) {
    use diesel::connection::SimpleConnection;
    use serde_json::json;
    let uri = "/api/v1/zsj/seznam";
    let path = "/zsj/seznam";
    let body = contract_response(app, uri, path, StatusCode::OK).await;
    let rows = body.as_array().unwrap();
    let expected = zsj::table
        .order(zsj::kod)
        .select(Zsj::as_select())
        .load::<Zsj>(connection)
        .unwrap();
    assert_eq!(rows.len(), 839);
    assert_eq!(rows.len(), expected.len());
    for (row, expected) in rows.iter().zip(&expected) {
        assert_eq!(row["kod"], expected.kod);
        assert_eq!(row["nazev"], expected.nazev);
        assert_eq!(row["lat"], expected.lat);
        assert_eq!(row["lon"], expected.lon);
        assert_eq!(row["kod_obce"], json!(expected.kod_obce));
        assert_eq!(row["boundary"]["type"], "Polygon");
        assert!(
            !row["boundary"]["coordinates"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(row["boundary"].get("crs").is_none());
    }
    assert_eq!(rows[0]["kod"], "000019");
    #[derive(QueryableByName)]
    struct GeometryCheck {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        matches: bool,
    }
    let check = diesel::sql_query(
        "SELECT ST_HausdorffDistance(boundary, ST_SetSRID(ST_GeomFromGeoJSON($1), 4326)) < 1e-12 AS matches FROM public.\"ZSJ\" WHERE kod = $2",
    )
    .bind::<diesel::sql_types::Text, _>(rows[0]["boundary"].to_string())
    .bind::<diesel::sql_types::Text, _>(rows[0]["kod"].as_str().unwrap())
    .get_result::<GeometryCheck>(connection)
    .unwrap();
    assert!(
        check.matches,
        "API polygon must preserve the stored boundary"
    );

    diesel::update(zsj::table.find("000019"))
        .set(zsj::kod_obce.eq(None::<String>))
        .execute(connection)
        .unwrap();
    let body = contract_response(app, uri, path, StatusCode::OK).await;
    assert!(body[0].get("kod_obce").unwrap().is_null());
    diesel::update(zsj::table.find("000019"))
        .set(zsj::kod_obce.eq(expected[0].kod_obce.clone()))
        .execute(connection)
        .unwrap();

    // DDL changes affect only this test's disposable container and retain all seeded data.
    connection
        .batch_execute("ALTER TABLE public.\"ZSJ\" RENAME TO zsj_test_saved")
        .unwrap();
    contract_response(app, uri, path, StatusCode::INTERNAL_SERVER_ERROR).await;
    connection
        .batch_execute("CREATE TABLE public.\"ZSJ\" (LIKE public.zsj_test_saved INCLUDING ALL)")
        .unwrap();
    assert_eq!(
        contract_response(app, uri, path, StatusCode::OK).await,
        json!([])
    );
    connection
        .batch_execute(
            "DROP TABLE public.\"ZSJ\"; ALTER TABLE public.zsj_test_saved RENAME TO \"ZSJ\"",
        )
        .unwrap();
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

    program_catalog_api(app, connection).await;

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

async fn program_catalog_api(app: &axum::Router, connection: &mut PgConnection) {
    use diesel::connection::SimpleConnection;
    use serde_json::{Value, json};
    use std::collections::BTreeSet;
    let body = contract_response(app, "/api/v1/obory", "/obory", StatusCode::OK).await;
    assert_eq!(body["meta"]["prumer_prihlasek_na_misto"], 2.66);
    assert_eq!(
        body["meta"]["prijimaci_rizeni"],
        json!({"rok":2026, "kolo":1})
    );
    assert!(body["meta"].get("max_min").is_none());
    assert!(body["meta"].get("scenar").is_none());
    // Check against raw offerings for every program, including form/level combinations.
    let offers = nabidka_oboru::table
        .select(NabidkaOboru::as_select())
        .load::<NabidkaOboru>(connection)
        .unwrap();
    for (uri, form, levels) in [
        ("/api/v1/obory", "den", vec![]),
        ("/api/v1/obory?forma=dal", "dal", vec![]),
        ("/api/v1/obory?stupen=H,M", "den", vec!["H", "M"]),
        ("/api/v1/obory?forma=dal&stupen=H", "dal", vec!["H"]),
    ] {
        let expected: Vec<_> = offers
            .iter()
            .filter(|offer| {
                offer.forma_studia == form
                    && (levels.is_empty() || levels.contains(&&offer.kod_oboru[6..7]))
            })
            .collect();
        let expected_codes: BTreeSet<_> = expected
            .iter()
            .map(|offer| offer.kod_oboru.as_str())
            .collect();
        let actual = contract_response(app, uri, "/obory", StatusCode::OK).await;
        let rows = actual["data"].as_array().unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row["kod"].as_str().unwrap())
                .collect::<BTreeSet<_>>(),
            expected_codes
        );
        assert_eq!(rows.len(), expected_codes.len());
        for row in rows {
            let matching: Vec<_> = expected
                .iter()
                .filter(|offer| row["kod"] == offer.kod_oboru)
                .collect();
            let capacity: i64 = matching
                .iter()
                .map(|offer| i64::from(offer.pocet_prijimanych))
                .sum();
            let applications: i64 = matching
                .iter()
                .map(|offer| i64::from(offer.loni_pocet_prihlasek))
                .sum();
            assert_eq!(row["kapacita"], capacity);
            assert_eq!(row["prihlasky"], applications);
            assert_eq!(
                row["pocet_skol"],
                matching
                    .iter()
                    .map(|offer| &offer.redizo)
                    .collect::<BTreeSet<_>>()
                    .len()
            );
            if capacity > 0 {
                let ratio = applications as f64 / capacity as f64;
                assert!((row["prihlasky_na_misto"].as_f64().unwrap() - ratio).abs() < 1e-9);
                assert!((row["index_pretlaku"].as_f64().unwrap() - ratio / 2.66).abs() < 1e-9);
            }
            for field in [
                "deti_v_dosahu",
                "deti_bez_oboru",
                "podil_deti_v_dosahu",
                "mist_na_100_deti",
            ] {
                assert!(row.get(field).is_none());
            }
        }
    }
    connection.batch_execute(r#"
        INSERT INTO "OBORY" (kod, nazev) VALUES
            ('99-99-H/04', 'Zero capacity'), ('99-99-H/05', 'No offerings');
        INSERT INTO "NABIDKA_OBORU" (id, redizo, kod_oboru, forma_studia, delka_studia, pocet_prijimanych, loni_pocet_prihlasek) VALUES
            ('99000000-0000-0000-0000-000000000001', '600008975', '99-99-H/01', 'den', 3, 5, 50),
            ('99000000-0000-0000-0000-000000000002', '600008975', '99-99-H/01', 'den', 3, 7, 70),
            ('99000000-0000-0000-0000-000000000003', '600022854', '99-99-H/01', 'den', 3, 0, 0),
            ('99000000-0000-0000-0000-000000000004', '600008975', '99-99-H/01', 'dal', 3, 3, 1),
            ('99000000-0000-0000-0000-000000000005', '600008975', '99-99-H/02', 'den', 3, 10, 0),
            ('99000000-0000-0000-0000-000000000006', '600008975', '99-99-H/03', 'den', 3, 10, 40),
            ('99000000-0000-0000-0000-000000000007', '600008975', '99-99-H/04', 'den', 3, 0, 1);
    "#).unwrap();
    let body = contract_response(app, "/api/v1/obory?stupen=H", "/obory", StatusCode::OK).await;
    let rows = body["data"].as_array().unwrap();
    let row = |code: &str| rows.iter().find(|row| row["kod"] == code).unwrap();
    let mapped = row("99-99-H/01");
    assert_eq!(mapped["pocet_skol"], 2);
    assert_eq!(mapped["kapacita"], 12);
    assert_eq!(mapped["prihlasky"], 120);
    assert_eq!(mapped["prihlasky_na_misto"], 10.0);
    // Two workplaces with the same ICO count as one company; tertiary jobs are excluded.
    assert_eq!(mapped["zamestnavatelu"], 1);
    assert_eq!(mapped["volna_mista"], 32);
    assert!((mapped["volna_mista_na_misto"].as_f64().unwrap() - 32.0 / 12.0).abs() < 1e-9);
    assert_eq!(mapped["signaly"], json!(["pretlak", "poptavka_trhu"]));
    for field in ["zamestnavatelu", "volna_mista", "volna_mista_na_misto"] {
        assert!(row("99-99-H/02").get(field).unwrap().is_null());
    }
    assert_eq!(row("99-99-H/02")["signaly"], json!(["nizky_zajem"]));
    assert_eq!(row("99-99-H/03")["volna_mista"], 0);
    assert_eq!(row("99-99-H/03")["zamestnavatelu"], 0);
    assert_eq!(row("99-99-H/03")["volna_mista_na_misto"], 0.0);
    assert!(row("99-99-H/04")["prihlasky_na_misto"].is_null());
    assert!(row("99-99-H/04")["index_pretlaku"].is_null());
    assert_eq!(row("99-99-H/04")["signaly"], json!([]));
    assert!(!rows.iter().any(|row| row["kod"] == "99-99-H/05"));
    let dal = contract_response(app, "/api/v1/obory?forma=dal", "/obory", StatusCode::OK).await;
    let mapped_dal = dal["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kod"] == "99-99-H/01")
        .unwrap();
    assert_eq!(mapped_dal["pocet_skol"], 1);
    assert_eq!(mapped_dal["kapacita"], 3);
    assert_eq!(mapped_dal["prihlasky"], 1);
    assert_eq!(mapped_dal["volna_mista"], 32);
    let ignored = contract_response(
        app,
        "/api/v1/obory?stupen=H&max_min=garbage&max_min=0&scenar=other",
        "/obory",
        StatusCode::OK,
    )
    .await;
    assert_eq!(ignored, body);
    for (sort, field, descending) in [
        ("index_pretlaku", "index_pretlaku", false),
        ("-index_pretlaku", "index_pretlaku", true),
        ("volna_mista_na_misto", "volna_mista_na_misto", false),
        ("-volna_mista_na_misto", "volna_mista_na_misto", true),
    ] {
        let sorted = contract_response(
            app,
            &format!("/api/v1/obory?razeni={sort}"),
            "/obory",
            StatusCode::OK,
        )
        .await;
        let rows = sorted["data"].as_array().unwrap();
        let mut null_seen = false;
        let mut previous = None;
        for row in rows {
            if let Some(value) = row[field].as_f64() {
                assert!(!null_seen, "null must sort last: {sort}");
                if let Some(previous) = previous {
                    assert!(if descending {
                        value <= previous
                    } else {
                        value >= previous
                    });
                }
                previous = Some(value);
            } else {
                null_seen = true;
            }
        }
    }
    for signals in [
        "pretlak",
        "nizky_zajem",
        "poptavka_trhu",
        "pretlak,poptavka_trhu",
    ] {
        let filtered = contract_response(
            app,
            &format!("/api/v1/obory?stupen=H&signal={signals}"),
            "/obory",
            StatusCode::OK,
        )
        .await;
        let expected: BTreeSet<_> = rows
            .iter()
            .filter(|row| {
                row["signaly"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|signal| signals.split(',').any(|wanted| signal == wanted))
            })
            .map(|row| row["kod"].as_str().unwrap())
            .collect();
        let actual: BTreeSet<_> = filtered["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["kod"].as_str().unwrap())
            .collect();
        assert_eq!(actual, expected);
    }
    // Default name ordering is deterministic even when numeric values tie.
    assert!(rows.windows(2).all(|pair| {
        let key = |row: &Value| {
            (
                row["nazev"].as_str().unwrap().to_lowercase(),
                row["kod"].as_str().unwrap().to_owned(),
            )
        };
        key(&pair[0]) <= key(&pair[1])
    }));
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

async fn zsj_accessibility_api(app: &axum::Router, connection: &mut PgConnection) {
    use diesel::connection::SimpleConnection;
    use serde_json::json;
    let path = "/zsj";
    let body = contract_response(app, "/api/v1/zsj?uroven=zsj", path, StatusCode::OK).await;
    let features = body["features"].as_array().unwrap();
    assert_eq!(features.len(), 839);
    assert_eq!(body["meta"]["jednotek_celkem"], 839);
    assert_eq!(body["meta"]["scenar"], "rano");
    assert_eq!(body["meta"]["max_min"], 120);
    assert_eq!(body["meta"]["uroven"], "zsj");
    #[derive(QueryableByName)]
    struct Expected {
        #[diesel(sql_type = diesel::sql_types::Text)]
        kod: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        geometry: String,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        populace: i32,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float>)]
        minutes: Option<f32>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        redizo: Option<String>,
    }
    let expected = diesel::sql_query(r#"SELECT z.kod, ST_AsGeoJSON(z.boundary, 15, 0) AS geometry,
        d.populace, nearest.doba_jizdy AS minutes, nearest.redizo::text AS redizo FROM "ZSJ" z
        JOIN "DATA_DEMOGRAFIE_ZSJ" d ON d.kod_zsj=z.kod AND d.rok=2021 AND d.demo_skupina='1300100014'
        LEFT JOIN LATERAL (SELECT t.doba_jizdy, t.redizo FROM "DOJEZDOVE_DOBY" t
            JOIN stredni_skoly s ON s.redizo=t.redizo
            WHERE t.kod_zsj=z.kod AND t.slot_prijezdu='07:00-08:00' AND t.doba_jizdy IS NOT NULL
            ORDER BY t.doba_jizdy, t.redizo LIMIT 1) nearest ON true ORDER BY z.kod"#)
        .load::<Expected>(connection).unwrap();
    for (feature, expected) in features.iter().zip(&expected) {
        let properties = &feature["properties"];
        assert_eq!(properties["kod"], expected.kod);
        assert_eq!(
            feature["geometry"],
            serde_json::from_str::<serde_json::Value>(&expected.geometry).unwrap()
        );
        assert_eq!(properties["deti"], (i64::from(expected.populace) + 2) / 5);
        assert_eq!(
            properties["v_dosahu"],
            expected.minutes.is_some_and(|v| v <= 120.0)
        );
        assert_eq!(
            properties.get("cas_min"),
            expected.minutes.map(|v| json!(v.round() as i64)).as_ref()
        );
        assert_eq!(
            properties.get("nejblizsi_redizo"),
            expected.redizo.as_ref().map(|v| json!(v)).as_ref()
        );
    }
    for item in body["meta"]["v_limitu"].as_array().unwrap() {
        let limit = item["limit_min"].as_i64().unwrap() as f32;
        assert_eq!(
            item["jednotek"],
            expected
                .iter()
                .filter(|e| e.minutes.is_some_and(|v| v <= limit))
                .count()
        );
    }
    let codes: Vec<String> = zsj::table
        .order(zsj::kod)
        .limit(9)
        .select(zsj::kod)
        .load(connection)
        .unwrap();
    let originals = dojezdove_doby::table
        .filter(dojezdove_doby::kod_zsj.eq_any(&codes))
        .filter(dojezdove_doby::slot_prijezdu.eq("07:00-08:00"))
        .select(DojezdovaDoba::as_select())
        .load::<DojezdovaDoba>(connection)
        .unwrap();
    let demo = data_demografie_zsj::table
        .find((&codes[0], 2021, "1300100014"))
        .select(DemografieZsj::as_select())
        .first::<DemografieZsj>(connection)
        .unwrap();
    connection.batch_execute(r#"INSERT INTO "OBORY" (kod,nazev) VALUES ('99-97-H/01','Reachability test');
        INSERT INTO "NABIDKA_OBORU" (id,redizo,kod_oboru,forma_studia,delka_studia,pocet_prijimanych,loni_pocet_prihlasek) VALUES
        ('97000000-0000-0000-0000-000000000001','600008975','99-97-H/01','den',3,5,10),
        ('97000000-0000-0000-0000-000000000002','600008975','99-97-H/01','den',3,5,10),
        ('97000000-0000-0000-0000-000000000003','600022854','99-97-H/01','den',3,5,10),
        ('97000000-0000-0000-0000-000000000004','600022854','99-97-H/01','dal',3,5,10);"#).unwrap();
    for (code, minutes) in codes.iter().zip([
        Some(0.0),
        Some(30.0),
        Some(30.1),
        Some(45.0),
        Some(45.1),
        Some(60.0),
        Some(60.4),
        Some(121.0),
        None,
    ]) {
        diesel::update(
            dojezdove_doby::table
                .filter(dojezdove_doby::kod_zsj.eq(code))
                .filter(dojezdove_doby::slot_prijezdu.eq("07:00-08:00")),
        )
        .set(dojezdove_doby::doba_jizdy.eq(1.0))
        .execute(connection)
        .unwrap();
        diesel::update(
            dojezdove_doby::table
                .filter(dojezdove_doby::kod_zsj.eq(code))
                .filter(dojezdove_doby::slot_prijezdu.eq("07:00-08:00"))
                .filter(dojezdove_doby::redizo.eq_any(["600008975", "600022854"])),
        )
        .set(dojezdove_doby::doba_jizdy.eq(minutes))
        .execute(connection)
        .unwrap();
    }
    diesel::update(data_demografie_zsj::table.find((&codes[0], 2021, "1300100014")))
        .set(data_demografie_zsj::populace.eq(13))
        .execute(connection)
        .unwrap();
    // Wrong arrival-slot data must not turn an unknown morning duration into a route.
    diesel::insert_into(dojezdove_doby::table)
        .values((
            dojezdove_doby::kod_zsj.eq(&codes[8]),
            dojezdove_doby::redizo.eq("600008975"),
            dojezdove_doby::slot_prijezdu.eq("09:00-10:00"),
            dojezdove_doby::doba_jizdy.eq(1.0),
        ))
        .execute(connection)
        .unwrap();
    let base = "/api/v1/zsj?uroven=zsj&obor=99-97-H%2F01";
    let body = contract_response(app, base, path, StatusCode::OK).await;
    for (index, band) in [
        "do30",
        "do30",
        "30_45",
        "30_45",
        "45_60",
        "45_60",
        "nad60",
        "mimo_dosah",
        "data_nedostupna",
    ]
    .iter()
    .enumerate()
    {
        let row = &body["features"][index]["properties"];
        assert_eq!(row["kod"], codes[index]);
        assert_eq!(row["pasmo"], *band);
        assert_eq!(row["v_dosahu"], index < 7);
        if index < 8 {
            assert_eq!(row["nejblizsi_redizo"], "600008975");
        }
    }
    assert_eq!(body["features"][0]["properties"]["deti"], 3);
    assert_eq!(body["features"][0]["properties"]["deti_v_dosahu"], 3);
    assert_eq!(
        body["features"][0]["properties"]["podil_deti_v_dosahu"],
        100.0
    );
    let unknown = &body["features"][8]["properties"];
    assert!(unknown.get("cas_min").is_none());
    assert!(unknown.get("nejblizsi_redizo").is_none());
    assert_eq!(unknown["deti_v_dosahu"], 0);
    let limited = contract_response(app, &format!("{base}&max_min=60"), path, StatusCode::OK).await;
    assert_eq!(limited["features"][6]["properties"]["cas_min"], 60);
    assert_eq!(limited["features"][6]["properties"]["pasmo"], "mimo_dosah");
    assert_eq!(limited["features"][6]["properties"]["v_dosahu"], false);
    assert_eq!(limited["meta"]["v_limitu"].as_array().unwrap().len(), 3);
    let short = contract_response(app, &format!("{base}&max_min=20"), path, StatusCode::OK).await;
    assert_eq!(short["features"][1]["properties"]["pasmo"], "mimo_dosah");
    let distance = contract_response(app, &format!("{base}&forma=dal"), path, StatusCode::OK).await;
    assert_eq!(
        distance["features"][0]["properties"]["nejblizsi_redizo"],
        "600022854"
    );
    let empty = contract_response(
        app,
        "/api/v1/zsj?uroven=zsj&obor=99-96-H%2F01",
        path,
        StatusCode::OK,
    )
    .await;
    assert!(
        empty["features"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["properties"]["pasmo"] == "bez_spojeni"
                && f["properties"]["v_dosahu"] == false)
    );
    assert!(
        empty["meta"]["v_limitu"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["jednotek"] == 0)
    );
    let missing = dojezdove_doby::table.find((&codes[0], "600008975", "07:00-08:00"));
    diesel::delete(missing).execute(connection).unwrap();
    contract_response(app, base, path, StatusCode::INTERNAL_SERVER_ERROR).await;
    diesel::insert_into(dojezdove_doby::table)
        .values((
            dojezdove_doby::kod_zsj.eq(&codes[0]),
            dojezdove_doby::redizo.eq("600008975"),
            dojezdove_doby::slot_prijezdu.eq("07:00-08:00"),
            dojezdove_doby::doba_jizdy.eq(0.0),
        ))
        .execute(connection)
        .unwrap();
    diesel::delete(data_demografie_zsj::table.find((&codes[0], 2021, "1300100014")))
        .execute(connection)
        .unwrap();
    contract_response(app, base, path, StatusCode::INTERNAL_SERVER_ERROR).await;
    diesel::insert_into(data_demografie_zsj::table)
        .values((
            data_demografie_zsj::kod_zsj.eq(&demo.kod_zsj),
            data_demografie_zsj::rok.eq(demo.rok),
            data_demografie_zsj::demo_skupina.eq(&demo.demo_skupina),
            data_demografie_zsj::populace.eq(demo.populace),
        ))
        .execute(connection)
        .unwrap();
    for row in originals {
        diesel::update(dojezdove_doby::table.find((&row.kod_zsj, &row.redizo, &row.slot_prijezdu)))
            .set(dojezdove_doby::doba_jizdy.eq(row.doba_jizdy))
            .execute(connection)
            .unwrap();
    }
    diesel::delete(dojezdove_doby::table.filter(dojezdove_doby::slot_prijezdu.eq("09:00-10:00")))
        .execute(connection)
        .unwrap();
    diesel::delete(nabidka_oboru::table.filter(nabidka_oboru::kod_oboru.eq("99-97-H/01")))
        .execute(connection)
        .unwrap();
    diesel::delete(obory::table.find("99-97-H/01"))
        .execute(connection)
        .unwrap();
}

async fn program_detail_api(app: &axum::Router, connection: &mut PgConnection) {
    use diesel::connection::SimpleConnection;
    use serde_json::json;
    let path = "/obory/{kod}";
    contract_response(
        app,
        "/api/v1/obory/99-96-H%2F99",
        path,
        StatusCode::NOT_FOUND,
    )
    .await;
    let seeded = contract_response(
        app,
        "/api/v1/obory/65-51-H%2F01?kandidatu=34",
        path,
        StatusCode::OK,
    )
    .await;
    let seeded_offers = nabidka_oboru::table
        .filter(nabidka_oboru::kod_oboru.eq("65-51-H/01"))
        .select(NabidkaOboru::as_select())
        .load::<NabidkaOboru>(connection)
        .unwrap();
    assert_eq!(
        seeded["nabidky"].as_array().unwrap().len(),
        seeded_offers.len()
    );
    assert_eq!(
        seeded["obor"]["kapacita"],
        seeded_offers
            .iter()
            .map(|o| i64::from(o.pocet_prijimanych))
            .sum::<i64>()
    );
    assert_eq!(
        seeded["obor"]["pocet_skol"],
        seeded_offers
            .iter()
            .map(|o| &o.redizo)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    // Independent SQL confirms union coverage rather than summing overlapping schools.
    #[derive(QueryableByName)]
    struct Coverage {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }
    let covered = diesel::sql_query(
        r#"SELECT COALESCE(sum((d.populace::bigint+2)/5),0)::bigint AS count
        FROM "DATA_DEMOGRAFIE_ZSJ" d WHERE d.rok=2021 AND d.demo_skupina='1300100014'
        AND EXISTS(SELECT 1 FROM "DOJEZDOVE_DOBY" t JOIN "NABIDKA_OBORU" o ON o.redizo=t.redizo
            WHERE t.kod_zsj=d.kod_zsj AND t.slot_prijezdu='07:00-08:00'
            AND t.doba_jizdy<=120 AND o.kod_oboru='65-51-H/01')"#,
    )
    .get_result::<Coverage>(connection)
    .unwrap();
    assert_eq!(seeded["obor"]["deti_v_dosahu"], covered.count);
    // Controlled fixtures isolate overlap, form inclusion, fractional cutoffs, NULLs and zero cohorts.
    connection.batch_execute(r#"INSERT INTO "OBORY" (kod,nazev) VALUES
        ('99-96-H/01','Detail test'),('99-96-H/02','No offerings'),('99-95-H/01','Related test');
        INSERT INTO "NABIDKA_OBORU" (id,redizo,kod_oboru,forma_studia,delka_studia,pocet_prijimanych,loni_pocet_prihlasek) VALUES
        ('96000000-0000-0000-0000-000000000001','600008975','99-96-H/01','den',3,5,50),
        ('96000000-0000-0000-0000-000000000002','600008975','99-96-H/01','den',3,7,70),
        ('96000000-0000-0000-0000-000000000003','600008975','99-96-H/01','dal',3,3,1),
        ('96000000-0000-0000-0000-000000000004','600022854','99-96-H/01','den',3,0,0),
        ('96000000-0000-0000-0000-000000000005','600009301','99-95-H/01','den',3,3,1);
        UPDATE "DATA_DEMOGRAFIE_ZSJ" SET populace=0 WHERE rok=2021 AND demo_skupina='1300100014';
        UPDATE "DOJEZDOVE_DOBY" SET doba_jizdy=NULL WHERE slot_prijezdu='07:00-08:00';
        UPDATE stredni_skoly SET nazev='Equal candidate' WHERE redizo IN ('600009301','600019632');"#).unwrap();
    let codes = zsj::table
        .order(zsj::kod)
        .limit(5)
        .select(zsj::kod)
        .load::<String>(connection)
        .unwrap();
    let schools = ["600008975", "600022854", "600009301", "600019632"];
    let durations = [
        [Some(30.0), Some(30.0), Some(0.0), Some(0.0)],
        [Some(60.4), None, Some(60.0), Some(60.0)],
        [Some(70.0), Some(60.0), Some(0.0), Some(0.0)],
        [None, None, Some(60.4), Some(60.0)],
        [Some(0.0), None, None, None],
    ];
    for (index, code) in codes.iter().enumerate() {
        diesel::update(data_demografie_zsj::table.find((code, 2021, "1300100014")))
            .set(data_demografie_zsj::populace.eq([50, 100, 150, 200, 0][index]))
            .execute(connection)
            .unwrap();
        for (school, time) in schools.iter().zip(durations[index]) {
            diesel::update(dojezdove_doby::table.find((code, *school, "07:00-08:00")))
                .set(dojezdove_doby::doba_jizdy.eq(time))
                .execute(connection)
                .unwrap();
        }
    }
    // An alternate slot does not fill NULLs in the morning matrix.
    diesel::insert_into(dojezdove_doby::table)
        .values((
            dojezdove_doby::kod_zsj.eq(&codes[3]),
            dojezdove_doby::redizo.eq(schools[0]),
            dojezdove_doby::slot_prijezdu.eq("09:00-10:00"),
            dojezdove_doby::doba_jizdy.eq(0.0),
        ))
        .execute(connection)
        .unwrap();
    let base = "/api/v1/obory/99-96-H%2F01?max_min=60";
    let body = contract_response(app, base, path, StatusCode::OK).await;
    let balance = &body["obor"];
    assert_eq!(balance["pocet_skol"], 2);
    assert_eq!(balance["kapacita"], 15);
    assert_eq!(balance["prihlasky"], 121);
    assert_eq!(balance["prihlasky_na_misto"], json!(121.0 / 15.0));
    assert_eq!(balance["deti_v_dosahu"], 40);
    assert_eq!(balance["deti_bez_oboru"], 60);
    assert_eq!(balance["podil_deti_v_dosahu"], 40.0);
    assert_eq!(balance["mist_na_100_deti"], 37.5);
    assert_eq!(balance["signaly"], json!(["pretlak", "spatna_dostupnost"]));
    assert_eq!(balance["volna_mista"], serde_json::Value::Null);
    assert_eq!(balance["zamestnavatelu"], serde_json::Value::Null);
    assert_eq!(body["trh_prace"], serde_json::Value::Null);
    assert_eq!(body["meta"], json!({"max_min":60,"scenar":"rano"}));
    let offers = body["nabidky"].as_array().unwrap();
    assert_eq!(offers.len(), 4);
    assert!(offers.iter().any(|o| o["forma"] == "dal"));
    for offer in offers {
        assert_eq!(
            offer["deti_v_dosahu_skoly"],
            if offer["redizo"] == schools[0] {
                10
            } else {
                40
            }
        );
    }
    let candidates = body["kandidati"].as_array().unwrap();
    assert_eq!(candidates.len(), 5);
    assert_eq!(candidates[0]["redizo"], schools[3]);
    assert_eq!(candidates[0]["nove_dosazene_deti"], 60);
    assert_eq!(candidates[1]["redizo"], schools[2]);
    assert_eq!(candidates[1]["nove_dosazene_deti"], 20);
    assert_eq!(candidates[1]["ma_pribuzny_obor"], true);
    assert_eq!(candidates[0]["ma_pribuzny_obor"], false);
    assert!(
        candidates
            .iter()
            .all(|c| c["redizo"] != schools[0] && c["redizo"] != schools[1])
    );
    let wider = contract_response(
        app,
        "/api/v1/obory/99-96-H%2F01?max_min=61&kandidatu=1",
        path,
        StatusCode::OK,
    )
    .await;
    assert_eq!(wider["obor"]["deti_v_dosahu"], 60);
    assert_eq!(wider["kandidati"].as_array().unwrap().len(), 1);
    assert_eq!(wider["kandidati"][0]["redizo"], schools[2]);
    assert_eq!(wider["kandidati"][0]["nove_dosazene_deti"], 40);
    connection
        .batch_execute(
            r#"INSERT INTO "OBOR_PROFESE" (cz_isco3,kod_oboru,vhodnost) VALUES
        ('991','99-96-H/01',1),('992','99-96-H/01',2),('993','99-96-H/02',1);"#,
        )
        .unwrap();
    let mapped = contract_response(app, base, path, StatusCode::OK).await;
    assert_eq!(mapped["obor"]["volna_mista"], 32);
    assert_eq!(mapped["obor"]["zamestnavatelu"], 1);
    assert_eq!(mapped["obor"]["volna_mista_na_misto"], json!(32.0 / 15.0));
    assert!(
        mapped["obor"]["signaly"]
            .as_array()
            .unwrap()
            .contains(&json!("poptavka_trhu"))
    );
    assert_eq!(mapped["trh_prace"]["profese"][0]["pocet"], 21);
    assert_eq!(mapped["trh_prace"]["profese"][1]["pocet"], 11);
    assert_eq!(mapped["trh_prace"]["profese"][0]["vhodnost"], 1);
    assert_eq!(mapped["trh_prace"]["profese"][1]["vhodnost"], 2);
    let empty = contract_response(
        app,
        "/api/v1/obory/99-96-H%2F02?max_min=60&kandidatu=34",
        path,
        StatusCode::OK,
    )
    .await;
    assert_eq!(empty["obor"]["pocet_skol"], 0);
    assert_eq!(empty["obor"]["deti_v_dosahu"], 0);
    assert_eq!(empty["obor"]["deti_bez_oboru"], 100);
    assert_eq!(empty["obor"]["prihlasky_na_misto"], serde_json::Value::Null);
    assert_eq!(empty["obor"]["mist_na_100_deti"], serde_json::Value::Null);
    assert_eq!(empty["obor"]["volna_mista"], 0);
    assert_eq!(empty["trh_prace"]["profese"][0]["pocet"], 0);
    assert_eq!(empty["kandidati"].as_array().unwrap().len(), 34);
    // Missing rows are errors; NULL durations above remain valid unknown data.
    let row = dojezdove_doby::table.find((&codes[0], schools[2], "07:00-08:00"));
    diesel::delete(row).execute(connection).unwrap();
    contract_response(app, base, path, StatusCode::INTERNAL_SERVER_ERROR).await;
    diesel::insert_into(dojezdove_doby::table)
        .values((
            dojezdove_doby::kod_zsj.eq(&codes[0]),
            dojezdove_doby::redizo.eq(schools[2]),
            dojezdove_doby::slot_prijezdu.eq("07:00-08:00"),
            dojezdove_doby::doba_jizdy.eq(0.0),
        ))
        .execute(connection)
        .unwrap();
    diesel::delete(data_demografie_zsj::table.find((&codes[0], 2021, "1300100014")))
        .execute(connection)
        .unwrap();
    contract_response(app, base, path, StatusCode::INTERNAL_SERVER_ERROR).await;
    contract_response(
        app,
        "/api/v1/obory/99-96-H%2F99",
        path,
        StatusCode::NOT_FOUND,
    )
    .await;
    diesel::insert_into(data_demografie_zsj::table)
        .values((
            data_demografie_zsj::kod_zsj.eq(&codes[0]),
            data_demografie_zsj::rok.eq(2021),
            data_demografie_zsj::demo_skupina.eq("1300100014"),
            data_demografie_zsj::populace.eq(0),
        ))
        .execute(connection)
        .unwrap();
    diesel::update(
        data_demografie_zsj::table
            .filter(data_demografie_zsj::rok.eq(2021))
            .filter(data_demografie_zsj::demo_skupina.eq("1300100014")),
    )
    .set(data_demografie_zsj::populace.eq(0))
    .execute(connection)
    .unwrap();
    let zero = contract_response(app, base, path, StatusCode::OK).await;
    assert_eq!(zero["obor"]["podil_deti_v_dosahu"], 0.0);
    assert_eq!(zero["obor"]["mist_na_100_deti"], serde_json::Value::Null);
    assert!(
        !zero["obor"]["signaly"]
            .as_array()
            .unwrap()
            .contains(&json!("spatna_dostupnost"))
    );
}

async fn simulation_api(app: &axum::Router, connection: &mut PgConnection) {
    use diesel::connection::SimpleConnection;
    use serde_json::json;
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type=diesel::sql_types::BigInt)]
        count: i64,
    }
    let count = diesel::sql_query(r#"SELECT count(*) FROM "SIMULATION_OBCE""#)
        .get_result::<Count>(connection)
        .unwrap();
    assert_eq!(count.count, 134);
    let count = diesel::sql_query(r#"SELECT count(DISTINCT kod_orp) FROM "SIMULATION_OBCE""#)
        .get_result::<Count>(connection)
        .unwrap();
    assert_eq!(count.count, 7);
    let base = "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=30";
    let path = "/simulace";
    let zsj_body =
        contract_response(app, &format!("{base}&uroven=zsj"), path, StatusCode::OK).await;
    assert_eq!(zsj_body["souhrn"]["jednotek_celkem"], 839);
    assert_eq!(zsj_body["meta"]["skola_obor_uz_uci"], false);
    assert!(zsj_body["souhrn"]["bilance_skol"].as_array().unwrap().len() > 1);
    let baseline = zsj_body["souhrn"]["bilance_skol"].clone();
    for (level, units) in [("zsj", 839), ("obec", 134), ("orp", 7)] {
        let dictionary =
            contract_response(app, &format!("{base}&uroven={level}"), path, StatusCode::OK).await;
        assert_eq!(dictionary["souhrn"]["jednotek_celkem"], units);
        assert_eq!(dictionary["souhrn"]["bilance_skol"], baseline);
        let geo = contract_response(
            app,
            &format!("{base}&uroven={level}&format=geojson"),
            path,
            StatusCode::OK,
        )
        .await;
        assert_eq!(geo["features"].as_array().unwrap().len(), units as usize);
        assert_eq!(geo["souhrn"], dictionary["souhrn"]);
        assert_eq!(geo["meta"], dictionary["meta"]);
        for feature in geo["features"].as_array().unwrap() {
            let properties = &feature["properties"];
            assert_eq!(properties["uroven"], level);
            let code = properties["kod"].as_str().unwrap();
            if let Some(improvement) = dictionary["jednotky"].get(code) {
                for (field, value) in improvement.as_object().unwrap() {
                    assert_eq!(&properties[field], value, "{level}/{code}/{field}");
                }
            } else {
                assert_eq!(properties["zlepseni_min"], 0.0);
            }
        }
    }
    contract_response(
        app,
        "/api/v1/simulace?redizo=600000000&obor=23-68-H%2F01&kapacita=30",
        path,
        StatusCode::NOT_FOUND,
    )
    .await;
    contract_response(
        app,
        "/api/v1/simulace?redizo=600009271&obor=99-94-H%2F99&kapacita=30",
        path,
        StatusCode::NOT_FOUND,
    )
    .await;
    connection
        .batch_execute(
            r#"INSERT INTO "OBORY" (kod,nazev) VALUES ('99-94-H/01','No simulation demand');"#,
        )
        .unwrap();
    contract_response(
        app,
        "/api/v1/simulace?redizo=600009271&obor=99-94-H%2F01&kapacita=30",
        path,
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    // All capacities below regional-average pressure should take the specified early exit.
    let quiet = nabidka_oboru::table
        .filter(nabidka_oboru::kod_oboru.eq("23-41-M/01"))
        .filter(nabidka_oboru::forma_studia.eq("den"))
        .order(nabidka_oboru::redizo)
        .select(nabidka_oboru::redizo)
        .first::<String>(connection)
        .unwrap();
    for format in ["slovnik", "geojson"] {
        let uri = format!(
            "/api/v1/simulace?redizo={quiet}&obor=23-41-M%2F01&kapacita=30&format={format}"
        );
        let body = contract_response(app, &uri, path, StatusCode::OK).await;
        assert!(body["souhrn"].is_null());
        assert_eq!(body["meta"]["duvod"], "kapacita_staci");
        if format == "geojson" {
            assert_eq!(body["features"], json!([]));
        } else {
            assert_eq!(body["jednotky"], json!({}));
        }
    }
    let missing = dojezdove_doby::table.find(("000019", "600009271", "07:00-08:00"));
    let time = missing
        .select(DojezdovaDoba::as_select())
        .first::<DojezdovaDoba>(connection)
        .unwrap();
    diesel::delete(missing).execute(connection).unwrap();
    contract_response(
        app,
        &format!("{base}&uroven=zsj"),
        path,
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await;
    diesel::insert_into(dojezdove_doby::table)
        .values((
            dojezdove_doby::kod_zsj.eq(&time.kod_zsj),
            dojezdove_doby::redizo.eq(&time.redizo),
            dojezdove_doby::slot_prijezdu.eq(&time.slot_prijezdu),
            dojezdove_doby::doba_jizdy.eq(time.doba_jizdy),
        ))
        .execute(connection)
        .unwrap();
    let demo = data_demografie_zsj::table.find(("000019", 2021, "1300100014"));
    let original = demo
        .select(data_demografie_zsj::populace)
        .first::<i32>(connection)
        .unwrap();
    diesel::delete(demo).execute(connection).unwrap();
    contract_response(
        app,
        &format!("{base}&uroven=zsj"),
        path,
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await;
    diesel::insert_into(data_demografie_zsj::table)
        .values((
            data_demografie_zsj::kod_zsj.eq("000019"),
            data_demografie_zsj::rok.eq(2021),
            data_demografie_zsj::demo_skupina.eq("1300100014"),
            data_demografie_zsj::populace.eq(original),
        ))
        .execute(connection)
        .unwrap();
    diesel::delete(obory::table.find("99-94-H/01"))
        .execute(connection)
        .unwrap();
}

async fn batch_simulation_api(app: &axum::Router, connection: &mut PgConnection) {
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    async fn post(app: &axum::Router, body: &Value, expected: StatusCode) -> Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/simulace/zmeny")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{body}");
        let media = if expected == StatusCode::OK && body["format"] == "geojson" {
            "application/geo+json"
        } else {
            "application/json"
        };
        assert_eq!(response.headers()["content-type"], media);
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 64_000_000).await.unwrap())
                .unwrap();
        let declared = contract::resolve(
            &contract::SPEC["paths"]["/simulace/zmeny"]["post"]["responses"][expected.as_str()],
        );
        let errors: Vec<_> = contract::schema_validator(&declared["content"][media]["schema"])
            .iter_errors(&value)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{}", errors.join("; "));
        value
    }
    fn offers(connection: &mut PgConnection) -> Vec<(String, String, String, i32, i32)> {
        nabidka_oboru::table
            .order(nabidka_oboru::id)
            .select((
                nabidka_oboru::redizo,
                nabidka_oboru::kod_oboru,
                nabidka_oboru::forma_studia,
                nabidka_oboru::pocet_prijimanych,
                nabidka_oboru::loni_pocet_prihlasek,
            ))
            .load(connection)
            .unwrap()
    }
    let before = offers(connection);
    let mut capacities = BTreeMap::<String, i32>::new();
    for (school, program, form, capacity, _) in &before {
        if program == "23-68-H/01" && form == "den" {
            *capacities.entry(school.clone()).or_default() += capacity;
        }
    }
    let (school, capacity) = capacities
        .iter()
        .find(|(_, c)| **c > 1 && **c <= 300)
        .unwrap();
    let old_uri = "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=30&uroven=zsj";
    let original = contract_response(app, old_uri, "/simulace", StatusCode::OK).await;
    let mut body = json!({"obor":"23-68-H/01","zmeny":[{"redizo":school,"zmena_kapacity":-capacity},{"redizo":"600009271","zmena_kapacity":capacity}],"uroven":"zsj"});
    let expected = post(app, &body, StatusCode::OK).await;
    assert_eq!(
        expected["souhrn"]["kapacita_pred"],
        expected["souhrn"]["kapacita_po"]
    );
    assert_eq!(expected["souhrn"]["jednotek_celkem"], 839);
    let effects = expected["souhrn"]["bilance_skol"].clone();
    body["zmeny"].as_array_mut().unwrap().reverse();
    assert_eq!(post(app, &body, StatusCode::OK).await, expected);
    for (level, count) in [("zsj", 839), ("obec", 134), ("orp", 7)] {
        body["uroven"] = json!(level);
        body["format"] = json!("slovnik");
        let dictionary = post(app, &body, StatusCode::OK).await;
        assert_eq!(dictionary["souhrn"]["jednotek_celkem"], count);
        assert_eq!(dictionary["souhrn"]["bilance_skol"], effects);
        body["format"] = json!("geojson");
        let geo = post(app, &body, StatusCode::OK).await;
        assert_eq!(geo["features"].as_array().unwrap().len(), count as usize);
        assert_eq!(geo["souhrn"], dictionary["souhrn"]);
        assert_eq!(geo["meta"], dictionary["meta"]);
        for feature in geo["features"].as_array().unwrap() {
            let p = &feature["properties"];
            if let Some(unit) = dictionary["jednotky"].get(p["kod"].as_str().unwrap()) {
                for (field, v) in unit.as_object().unwrap() {
                    assert_eq!(&p[field], v);
                }
            }
        }
    }
    let partial =
        json!({"obor":"23-68-H/01","uroven":"zsj","zmeny":[{"redizo":school,"zmena_kapacity":-1}]});
    let v = post(app, &partial, StatusCode::OK).await;
    assert_eq!(
        v["souhrn"]["kapacita_pred"].as_i64().unwrap() - 1,
        v["souhrn"]["kapacita_po"].as_i64().unwrap()
    );
    // Positive remaining capacity retains all existing routes and catchments.
    assert_eq!(v["jednotky"], json!({}));
    assert_eq!(v["souhrn"]["presuny"], json!([]));
    let removal = json!({"obor":"23-68-H/01","uroven":"zsj","zmeny":capacities.iter().filter(|(_,c)|**c>0).map(|(s,c)|json!({"redizo":s,"zmena_kapacity":-c})).collect::<Vec<_>>()});
    let v = post(app, &removal, StatusCode::OK).await;
    assert_eq!(v["souhrn"]["kapacita_po"], 0);
    assert_eq!(v["souhrn"]["deti_v_dosahu_po"], 0.0);
    assert!(v["souhrn"]["ztracene_deti"].as_f64().unwrap() > 0.0);
    post(
        app,
        &json!({"obor":"23-68-H/01","zmeny":[{"redizo":school,"zmena_kapacity":-300}]}),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    post(
        app,
        &json!({"obor":"23-68-H/01","zmeny":[{"redizo":"600000000","zmena_kapacity":1}]}),
        StatusCode::NOT_FOUND,
    )
    .await;
    post(
        app,
        &json!({"obor":"99-94-H/99","zmeny":[{"redizo":school,"zmena_kapacity":1}]}),
        StatusCode::NOT_FOUND,
    )
    .await;
    let missing = dojezdove_doby::table.find(("000019", "600009271", "07:00-08:00"));
    let time = missing
        .select(DojezdovaDoba::as_select())
        .first::<DojezdovaDoba>(connection)
        .unwrap();
    diesel::delete(missing).execute(connection).unwrap();
    post(app, &body, StatusCode::INTERNAL_SERVER_ERROR).await;
    diesel::insert_into(dojezdove_doby::table)
        .values((
            dojezdove_doby::kod_zsj.eq(&time.kod_zsj),
            dojezdove_doby::redizo.eq(&time.redizo),
            dojezdove_doby::slot_prijezdu.eq(&time.slot_prijezdu),
            dojezdove_doby::doba_jizdy.eq(time.doba_jizdy),
        ))
        .execute(connection)
        .unwrap();
    assert_eq!(
        offers(connection),
        before,
        "Simulation must not write admissions data"
    );
    assert_eq!(
        contract_response(app, old_uri, "/simulace", StatusCode::OK).await,
        original
    );
}
