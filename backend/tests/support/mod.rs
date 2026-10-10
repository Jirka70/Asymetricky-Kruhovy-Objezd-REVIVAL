use axum::{Json, Router, http::StatusCode, routing::post};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate};
use obor_backend::otp::{Client, Config};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration as StdDuration,
};

pub struct MockOtp {
    pub client: Client,
    pub requests: Arc<Mutex<Vec<Value>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for MockOtp {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl MockOtp {
    pub async fn start(
        handler: impl Fn(&Value) -> (StatusCode, Value) + Send + Sync + 'static,
    ) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let capture = requests.clone();
        let handler = Arc::new(handler);
        let app = Router::new().route(
            "/otp/gtfs/v1",
            post(move |Json(body): Json<Value>| {
                let handler = handler.clone();
                let capture = capture.clone();
                async move {
                    capture.lock().unwrap().push(body.clone());
                    let (status, value) = handler(&body);
                    (status, Json(value))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = Client::new(Config {
            url: format!("http://{address}/otp/gtfs/v1"),
            service_date: Some(NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()),
            timeout: StdDuration::from_secs(2),
            concurrency: 2,
        })
        .unwrap();
        Self {
            client,
            requests,
            task,
        }
    }
}
fn encode(points: &[(f64, f64)]) -> String {
    let mut output = String::new();
    let mut last_lat = 0i64;
    let mut last_lon = 0i64;
    for &(lon, lat) in points {
        let lat = (lat * 100_000.0).round() as i64;
        let lon = (lon * 100_000.0).round() as i64;
        for delta in [lat - last_lat, lon - last_lon] {
            let mut value = if delta < 0 { !(delta << 1) } else { delta << 1 };
            while value >= 32 {
                output.push(char::from(((value & 31) | 32) as u8 + 63));
                value >>= 5;
            }
            output.push(char::from(value as u8 + 63));
        }
        last_lat = lat;
        last_lon = lon;
    }
    output
}
pub fn itinerary(
    minutes: f64,
    arrival: &str,
    origin: (f64, f64),
    destination: (f64, f64),
) -> Value {
    let end = DateTime::<FixedOffset>::parse_from_rfc3339(arrival).unwrap();
    let start = end - Duration::milliseconds((minutes * 60_000.0).round() as i64);
    json!({"start":start.to_rfc3339(),"end":end.to_rfc3339(),"duration":minutes*60.0,"numberOfTransfers":0,"walkDistance":123.6,"legs":[{"mode":"WALK","distance":123.6,"start":{"scheduledTime":start.to_rfc3339(),"estimated":{"time":start.to_rfc3339()}},"end":{"scheduledTime":end.to_rfc3339(),"estimated":{"time":end.to_rfc3339()}},"from":{"name":"Origin"},"to":{"name":"School"},"route":null,"legGeometry":{"points":encode(&[(origin.1,origin.0),(destination.1,destination.0)])}}]})
}
pub fn response(itineraries: Vec<Value>) -> Value {
    json!({"data":{"planConnection":{"routingErrors":[],"edges":itineraries.into_iter().map(|node|json!({"node":node})).collect::<Vec<_>>()}}})
}
pub fn origin(body: &Value) -> (f64, f64) {
    let p = &body["variables"]["origin"]["location"]["coordinate"];
    (
        p["latitude"].as_f64().unwrap(),
        p["longitude"].as_f64().unwrap(),
    )
}
pub fn destination(body: &Value) -> (f64, f64) {
    let p = &body["variables"]["destination"]["location"]["coordinate"];
    (
        p["latitude"].as_f64().unwrap(),
        p["longitude"].as_f64().unwrap(),
    )
}
