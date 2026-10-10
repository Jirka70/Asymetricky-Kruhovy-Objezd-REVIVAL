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
    assert_eq!(db::migrate(&mut connection).unwrap(), 2);
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
    zsj_api(&app, &mut connection).await;
    catalog_api(&app, &mut connection).await;

    assert_eq!(
        connection
            .revert_all_migrations(db::MIGRATIONS)
            .unwrap()
            .len(),
        9
    );
    let remaining = diesel::sql_query("SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tablename IN ('ZSJ', 'DEMO_SKUPINA', 'DATA_DEMOGRAFIE_ZSJ', 'DOJEZDOVE_DOBY')")
        .get_result::<Count>(&mut connection).unwrap();
    assert_eq!(remaining.count, 0);
    assert_eq!(db::migrate(&mut connection).unwrap(), 9);
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
    let media =
        if status == StatusCode::OK && matches!(path, "/skoly" | "/obory/{kod}/zamestnavatele") {
            "application/geo+json"
        } else {
            "application/json"
        };
    assert_eq!(response.headers()["content-type"], media);
    let body_limit = if path == "/zsj/seznam" {
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
