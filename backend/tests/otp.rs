mod support;
use axum::{http::StatusCode, response::IntoResponse};
use chrono::{NaiveDate, TimeZone, Utc};
use obor_backend::{
    contract,
    otp::{Client, Config, next_school_day},
};
use serde_json::json;
use std::time::Duration;
const ORIGIN: (f64, f64) = (50.2312, 12.8711);
const TARGET: (f64, f64) = (50.24, 12.88);
fn date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()
}
async fn status(error: obor_backend::contract::StubError) -> StatusCode {
    error.into_response().status()
}
#[tokio::test]
async fn chooses_three_shortest_same_day_routes_in_arrival_window_and_decodes_geometry() {
    let mock = support::MockOtp::start(|_| {
        (
            StatusCode::OK,
            support::response(vec![
                support::itinerary(40.0, "2026-10-12T07:40:00+02:00", ORIGIN, TARGET),
                support::itinerary(10.0, "2026-10-12T08:00:00+02:00", ORIGIN, TARGET),
                support::itinerary(30.0, "2026-10-12T07:30:00+02:00", ORIGIN, TARGET),
                support::itinerary(20.0, "2026-10-12T07:45:00+02:00", ORIGIN, TARGET),
                support::itinerary(20.0, "2026-10-12T07:55:00+02:00", ORIGIN, TARGET),
                support::itinerary(10.0, "2026-10-12T06:59:59+02:00", ORIGIN, TARGET),
                support::itinerary(600.0, "2026-10-12T07:50:00+02:00", ORIGIN, TARGET),
            ]),
        )
    })
    .await;
    let routes = mock.client.routes(ORIGIN, TARGET, date()).await.unwrap();
    assert_eq!(routes.itineraries.len(), 3);
    assert_eq!(
        routes
            .itineraries
            .iter()
            .map(|i| i.duration_seconds)
            .collect::<Vec<_>>(),
        vec![1200.0, 1200.0, 1800.0]
    );
    let geo = routes.geojson().unwrap();
    assert_eq!(geo.spoje[0].prijezd, "07:55");
    assert_eq!(geo.spoje[0].rezerva_min, Some(5));
    assert_eq!(
        geo.features[0].geometry.coordinates,
        vec![vec![ORIGIN.1, ORIGIN.0], vec![TARGET.1, TARGET.0]]
    );
    assert_eq!(geo.features[0].properties.chuze_m, Some(124));
    assert_eq!(routes.summary().chuze_m, Some(124));
    assert_eq!(routes.summary().vzdalenost_m, Some(124));
    let value = serde_json::to_value(geo).unwrap();
    let schema = &contract::SPEC["components"]["schemas"]["Trasa"];
    assert!(contract::schema_validator(schema).is_valid(&value));
    let req = mock.requests.lock().unwrap();
    assert_eq!(support::origin(&req[0]), ORIGIN);
    assert_eq!(support::destination(&req[0]), TARGET);
    assert_eq!(
        req[0]["variables"]["dateTime"]["latestArrival"],
        "2026-10-12T07:59:59+02:00"
    );
    assert!(req[0]["query"].as_str().unwrap().contains("legGeometry"));
}
#[tokio::test]
async fn caches_success_and_no_route_coalesces_concurrent_requests_and_keys_exact_points_and_dates()
{
    let mock = support::MockOtp::start(|_| (StatusCode::OK, support::response(vec![]))).await;
    let (a, b) = tokio::join!(
        mock.client.routes(ORIGIN, TARGET, date()),
        mock.client.routes(ORIGIN, TARGET, date())
    );
    assert!(a.unwrap().itineraries.is_empty());
    assert!(b.unwrap().itineraries.is_empty());
    mock.client.routes(ORIGIN, TARGET, date()).await.unwrap();
    assert_eq!(mock.requests.lock().unwrap().len(), 1);
    mock.client
        .routes((ORIGIN.0 + 0.00001, ORIGIN.1), TARGET, date())
        .await
        .unwrap();
    mock.client
        .routes(ORIGIN, TARGET, date().succ_opt().unwrap())
        .await
        .unwrap();
    assert_eq!(mock.requests.lock().unwrap().len(), 3);
}
#[tokio::test]
async fn maps_upstream_failures_to_502_without_caching_them() {
    for body in [
        json!({"errors":[{"message":"bad query"}],"data":null}),
        json!({"data":{"planConnection":null}}),
        json!({"data":{"planConnection":{"routingErrors":[{"code":"OUTSIDE_SERVICE_PERIOD"}],"edges":[]}}}),
        json!({"data":{"planConnection":{"routingErrors":[{"code":"UNKNOWN_ERROR"}],"edges":[]}}}),
    ] {
        let mock = support::MockOtp::start(move |_| (StatusCode::OK, body.clone())).await;
        for _ in 0..2 {
            assert_eq!(
                status(
                    mock.client
                        .routes(ORIGIN, TARGET, date())
                        .await
                        .unwrap_err()
                )
                .await,
                StatusCode::BAD_GATEWAY
            );
        }
        assert_eq!(mock.requests.lock().unwrap().len(), 2);
    }
    let mock = support::MockOtp::start(|_| (StatusCode::SERVICE_UNAVAILABLE, json!({}))).await;
    assert!(mock.client.routes(ORIGIN, TARGET, date()).await.is_err());
    let client = Client::new(Config {
        url: "http://127.0.0.1:1/otp/gtfs/v1".into(),
        timeout: Duration::from_millis(50),
        ..Config::default()
    })
    .unwrap();
    assert_eq!(
        status(client.routes(ORIGIN, TARGET, date()).await.unwrap_err()).await,
        StatusCode::BAD_GATEWAY
    );
}
#[tokio::test]
async fn validates_geometry_times_and_mode_and_recognizes_no_connection_errors() {
    for (pointer, value) in [
        ("/legs/0/legGeometry/points", json!("?")),
        ("/legs/0/mode", json!("UNKNOWN")),
        ("/duration", json!(-1.0)),
        ("/walkDistance", json!(-1.0)),
        ("/numberOfTransfers", json!(-1)),
        (
            "/legs/0/start/estimated/time",
            json!("2026-10-12T09:00:00+02:00"),
        ),
    ] {
        let mut i = support::itinerary(20.0, "2026-10-12T07:50:00+02:00", ORIGIN, TARGET);
        *i.pointer_mut(pointer).unwrap() = value;
        let mock =
            support::MockOtp::start(move |_| (StatusCode::OK, support::response(vec![i.clone()])))
                .await;
        assert!(mock.client.routes(ORIGIN, TARGET, date()).await.is_err());
    }
    let mock=support::MockOtp::start(|_|(StatusCode::OK,json!({"data":{"planConnection":{"routingErrors":[{"code":"NO_TRANSIT_CONNECTION"}],"edges":[]}}}))).await;
    assert_eq!(
        serde_json::to_value(
            mock.client
                .routes(ORIGIN, TARGET, date())
                .await
                .unwrap()
                .summary()
        )
        .unwrap()["stav"],
        "bez_spojeni"
    );
}
#[test]
fn service_day_uses_prague_weekdays_and_morning_cutoff_including_dst() {
    for (now, expected) in [
        ("2026-10-09T05:59:59Z", "2026-10-09"),
        ("2026-10-09T06:00:00Z", "2026-10-12"),
        ("2026-10-10T10:00:00Z", "2026-10-12"),
        ("2026-10-26T06:59:59Z", "2026-10-26"),
        ("2026-10-26T07:00:00Z", "2026-10-27"),
    ] {
        let now = chrono::DateTime::parse_from_rfc3339(now)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(next_school_day(now).to_string(), expected);
    }
    assert_eq!(
        next_school_day(Utc.with_ymd_and_hms(2026, 10, 12, 4, 0, 0).unwrap()),
        date()
    );
}

#[tokio::test]
async fn scheduled_times_are_used_when_otp_has_no_realtime_estimates() {
    let mut itinerary = support::itinerary(20.0, "2026-10-12T07:50:00+02:00", ORIGIN, TARGET);
    itinerary["legs"][0]["start"]["estimated"] = serde_json::Value::Null;
    itinerary["legs"][0]["end"]["estimated"] = serde_json::Value::Null;
    let mock = support::MockOtp::start(move |_| {
        (StatusCode::OK, support::response(vec![itinerary.clone()]))
    })
    .await;
    let body = mock
        .client
        .routes(ORIGIN, TARGET, date())
        .await
        .unwrap()
        .geojson()
        .unwrap();
    assert_eq!(body.features[0].properties.odjezd, "07:30");
    assert_eq!(body.features[0].properties.prijezd, "07:50");
}

#[tokio::test]
async fn otp_concurrency_is_bounded_and_timeouts_and_invalid_json_return_502() {
    use axum::{Json, Router, routing::post};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let a = active.clone();
    let p = peak.clone();
    let c = calls.clone();
    let app = Router::new()
        .route(
            "/route",
            post(move || {
                let a = a.clone();
                let p = p.clone();
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    let n = a.fetch_add(1, Ordering::SeqCst) + 1;
                    p.fetch_max(n, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    a.fetch_sub(1, Ordering::SeqCst);
                    Json(support::response(vec![]))
                }
            }),
        )
        .route("/invalid", post(|| async { "{" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let config = Config {
        url: format!("http://{address}/route"),
        timeout: Duration::from_secs(2),
        concurrency: 2,
        ..Config::default()
    };
    let client = Client::new(config.clone()).unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..8 {
        let client = client.clone();
        tasks.spawn(async move {
            client
                .routes((ORIGIN.0 + index as f64 * 0.0001, ORIGIN.1), TARGET, date())
                .await
        });
    }
    while let Some(result) = tasks.join_next().await {
        result.unwrap().unwrap();
    }
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    let timeout = Client::new(Config {
        timeout: Duration::from_millis(5),
        ..config.clone()
    })
    .unwrap();
    assert_eq!(
        status(timeout.routes(ORIGIN, TARGET, date()).await.unwrap_err()).await,
        StatusCode::BAD_GATEWAY
    );
    let invalid = Client::new(Config {
        url: format!("http://{address}/invalid"),
        ..config
    })
    .unwrap();
    assert_eq!(
        status(invalid.routes(ORIGIN, TARGET, date()).await.unwrap_err()).await,
        StatusCode::BAD_GATEWAY
    );
    server.abort();
}

#[tokio::test]
async fn nonadjacent_duplicate_candidates_do_not_consume_distinct_journey_slots() {
    let a = support::itinerary(20.0, "2026-10-12T07:50:00+02:00", ORIGIN, TARGET);
    let mut b = a.clone();
    b["legs"][0]["route"] = json!({"shortName":"Other","longName":null});
    let c = support::itinerary(30.0, "2026-10-12T07:55:00+02:00", ORIGIN, TARGET);
    let mock = support::MockOtp::start(move |_| {
        (
            StatusCode::OK,
            support::response(vec![a.clone(), b.clone(), a.clone(), c.clone()]),
        )
    })
    .await;
    let routes = mock.client.routes(ORIGIN, TARGET, date()).await.unwrap();
    assert_eq!(routes.itineraries.len(), 3);
    assert_eq!(routes.itineraries[2].duration_seconds, 1800.0);
}

#[tokio::test]
async fn coach_routes_map_to_bus_in_the_public_api() {
    let mut itinerary = support::itinerary(20.0, "2026-10-12T07:50:00+02:00", ORIGIN, TARGET);
    itinerary["walkDistance"] = json!(0.0);
    itinerary["legs"][0]["mode"] = json!("COACH");
    itinerary["legs"][0]["distance"] = json!(8000.0);
    itinerary["legs"][0]["route"] = json!({"shortName":"420309","longName":"Intercity coach"});
    let mock = support::MockOtp::start(move |_| {
        (StatusCode::OK, support::response(vec![itinerary.clone()]))
    })
    .await;
    let routes = mock.client.routes(ORIGIN, TARGET, date()).await.unwrap();
    let geo = serde_json::to_value(routes.geojson().unwrap()).unwrap();
    assert_eq!(geo["features"][0]["properties"]["druh"], "BUS");
    assert_eq!(geo["features"][0]["properties"]["linka"], "420309");
    assert!(geo["features"][0]["properties"].get("chuze_m").is_none());
    assert!(
        contract::schema_validator(&contract::SPEC["components"]["schemas"]["Trasa"])
            .is_valid(&geo)
    );
    let summary = serde_json::to_value(routes.summary()).unwrap();
    assert_eq!(summary["stav"], "ok");
    assert_eq!(summary["linky"], json!(["420309"]));
    assert_eq!(summary["vzdalenost_m"], 8000);
    assert!(
        contract::schema_validator(&contract::SPEC["components"]["schemas"]["Spoj"])
            .is_valid(&summary)
    );
}
