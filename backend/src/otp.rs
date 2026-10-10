//! Shared OTP GTFS GraphQL client for student routes and school travel summaries.
use crate::{
    contract::{StubError, api_error},
    dto,
};
use axum::http::StatusCode;
use chrono::{
    DateTime, Datelike, Duration as ChronoDuration, NaiveDate, TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Europe::Prague;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, OnceCell, Semaphore};

const QUERY: &str = r#"query StudentRoute($origin: PlanLabeledLocationInput!, $destination: PlanLabeledLocationInput!, $dateTime: PlanDateTimeInput!, $first: Int!, $window: Duration!) {
  planConnection(origin:$origin,destination:$destination,dateTime:$dateTime,first:$first,searchWindow:$window) {
    routingErrors { code }
    edges { node { start end duration numberOfTransfers walkDistance
      legs { mode distance start { scheduledTime estimated { time } } end { scheduledTime estimated { time } }
        route { shortName longName } from { name } to { name } legGeometry { points } }
    } }
  }
}"#;
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const CACHE_SIZE: usize = 1024;
#[derive(Clone, Debug)]
pub struct Config {
    pub url: String,
    pub service_date: Option<NaiveDate>,
    pub timeout: Duration,
    pub concurrency: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080/otp/gtfs/v1".into(),
            service_date: None,
            timeout: Duration::from_secs(20),
            concurrency: 4,
        }
    }
}
impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();
        if let Ok(url) = std::env::var("OTP_URL") {
            config.url = url;
        }
        if let Some(date) = std::env::var("OTP_SERVICE_DATE")
            .ok()
            .filter(|date| !date.is_empty())
        {
            config.service_date = Some(NaiveDate::parse_from_str(&date, "%Y-%m-%d")?);
        }
        Ok(config)
    }
}
#[derive(Clone)]
pub struct Client(Arc<Inner>);
type Key = (u64, u64, u64, u64, NaiveDate);
type Cached = Arc<OnceCell<Arc<RouteSet>>>;
struct Inner {
    http: reqwest::Client,
    config: Config,
    permits: Semaphore,
    cache: Mutex<BTreeMap<Key, (Instant, Cached)>>,
}
impl Default for Client {
    fn default() -> Self {
        Self::new(Config::default()).expect("valid default OTP configuration")
    }
}
impl Client {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let url = reqwest::Url::parse(&config.url)?;
        anyhow::ensure!(
            matches!(url.scheme(), "http" | "https"),
            "OTP_URL must be an HTTP(S) GraphQL URL"
        );
        anyhow::ensure!(
            config.concurrency > 0 && !config.timeout.is_zero(),
            "OTP concurrency and timeout must be positive"
        );
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .connect_timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self(Arc::new(Inner {
            http,
            permits: Semaphore::new(config.concurrency),
            config,
            cache: Mutex::new(BTreeMap::new()),
        })))
    }
    pub fn service_date(&self) -> NaiveDate {
        self.0
            .config
            .service_date
            .unwrap_or_else(|| next_school_day(Utc::now()))
    }
    pub async fn routes(
        &self,
        origin: (f64, f64),
        destination: (f64, f64),
        date: NaiveDate,
    ) -> Result<Arc<RouteSet>, StubError> {
        for (lat, lon) in [origin, destination] {
            if !lat.is_finite()
                || !lon.is_finite()
                || !(-90.0..=90.0).contains(&lat)
                || !(-180.0..=180.0).contains(&lon)
            {
                return Err(unavailable());
            }
        }
        // Exact coordinates avoid reusing a route whose start is a different street.
        let key = (
            origin.0.to_bits(),
            origin.1.to_bits(),
            destination.0.to_bits(),
            destination.1.to_bits(),
            date,
        );
        let cell = {
            let mut cache = self.0.cache.lock().await;
            cache.retain(|_, (created, _)| created.elapsed() < CACHE_TTL);
            if !cache.contains_key(&key) && cache.len() >= CACHE_SIZE {
                if let Some(oldest) = cache.iter().min_by_key(|(_, v)| v.0).map(|(k, _)| *k) {
                    cache.remove(&oldest);
                }
            }
            cache
                .entry(key)
                .or_insert_with(|| (Instant::now(), Arc::new(OnceCell::new())))
                .1
                .clone()
        };
        let routes = tokio::time::timeout(
            Duration::from_secs(60),
            cell.get_or_try_init(|| async {
                self.fetch(origin, destination, date).await.map(Arc::new)
            }),
        )
        .await
        .map_err(|_| unavailable())??;
        Ok(routes.clone())
    }
    async fn fetch(
        &self,
        origin: (f64, f64),
        destination: (f64, f64),
        date: NaiveDate,
    ) -> Result<RouteSet, StubError> {
        let _permit = self.0.permits.acquire().await.map_err(|_| unavailable())?;
        let (_, end) = window(date)?;
        let location =
            |(lat, lon)| json!({"location":{"coordinate":{"latitude":lat,"longitude":lon}}});
        let body = json!({"query":QUERY,"variables":{"origin":location(origin),"destination":location(destination),"dateTime":{"latestArrival":(end-ChronoDuration::seconds(1)).to_rfc3339()},"first":20,"window":"PT120M"}});
        let mut response = self
            .0
            .http
            .post(&self.0.config.url)
            .json(&body)
            .send()
            .await
            .map_err(|_| unavailable())?
            .error_for_status()
            .map_err(|_| unavailable())?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len() + chunk.len() > 8_000_000 {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        let response: OtpResponse = serde_json::from_slice(&bytes).map_err(|error| {
            tracing::warn!(%error, "Invalid OTP GraphQL response");
            unavailable()
        })?;
        parse(response, date)
    }
}
pub fn next_school_day(now: DateTime<Utc>) -> NaiveDate {
    let local = now.with_timezone(&Prague);
    let mut day = local.date_naive();
    if local.hour() >= 8 {
        day = day.succ_opt().expect("current date has a successor");
    }
    while matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
        day = day.succ_opt().expect("current date has a successor");
    }
    day
}
fn window(
    date: NaiveDate,
) -> Result<(DateTime<chrono_tz::Tz>, DateTime<chrono_tz::Tz>), StubError> {
    let time = |hour| {
        Prague
            .with_ymd_and_hms(date.year(), date.month(), date.day(), hour, 0, 0)
            .single()
            .ok_or_else(unavailable)
    };
    Ok((time(7)?, time(8)?))
}
fn unavailable() -> StubError {
    api_error(
        StatusCode::BAD_GATEWAY,
        "otp_nedostupne",
        "OpenTripPlanner je nedostupný nebo neposkytl platná data pro požadovaný den.",
    )
}
#[derive(Debug, Clone)]
pub struct RouteSet {
    pub date: NaiveDate,
    pub itineraries: Vec<Itinerary>,
}
#[derive(Debug, Clone)]
pub struct Itinerary {
    pub start: DateTime<chrono_tz::Tz>,
    pub end: DateTime<chrono_tz::Tz>,
    pub duration_seconds: f64,
    pub transfers: i64,
    pub walk_meters: f64,
    pub legs: Vec<Leg>,
}
#[derive(Debug, Clone)]
pub struct Leg {
    pub mode: dto::TrasaFeaturesItemPropertiesDruh,
    pub distance: f64,
    pub start: DateTime<chrono_tz::Tz>,
    pub end: DateTime<chrono_tz::Tz>,
    pub from: String,
    pub to: String,
    pub line: Option<String>,
    pub coordinates: Vec<Vec<f64>>,
}
impl Itinerary {
    fn distance_meters(&self) -> f64 {
        self.legs.iter().map(|leg| leg.distance).sum()
    }
}
impl RouteSet {
    pub fn summary(&self) -> dto::Spoj {
        match self.itineraries.first() {
            None => dto::Spoj {
                stav: dto::SpojStav::BezSpojeni,
                odjezd: None,
                prijezd: None,
                cas_min: None,
                prestupy: None,
                chuze_m: None,
                vzdalenost_m: None,
                linky: None,
            },
            Some(i) => dto::Spoj {
                stav: dto::SpojStav::Ok,
                odjezd: Some(i.start.format("%H:%M").to_string()),
                prijezd: Some(i.end.format("%H:%M").to_string()),
                cas_min: Some((i.duration_seconds / 60.0).round() as i64),
                prestupy: Some(i.transfers),
                chuze_m: Some(i.walk_meters.round() as i64),
                vzdalenost_m: Some(i.distance_meters().round() as i64),
                linky: Some(i.legs.iter().filter_map(|l| l.line.clone()).collect()),
            },
        }
    }
    pub fn geojson(&self) -> Result<dto::Trasa, StubError> {
        let (_, deadline) = window(self.date)?;
        let mut features = Vec::new();
        let mut spoje = Vec::new();
        for (index, itinerary) in self.itineraries.iter().enumerate() {
            let spoj = index as i64;
            spoje.push(dto::TrasaSpojeItem {
                spoj,
                odjezd: itinerary.start.format("%H:%M").to_string(),
                prijezd: itinerary.end.format("%H:%M").to_string(),
                cas_min: (itinerary.duration_seconds / 60.0).round() as i64,
                prestupy: itinerary.transfers,
                chuze_m: Some(itinerary.walk_meters.round() as i64),
                vzdalenost_m: Some(itinerary.distance_meters().round() as i64),
                rezerva_min: Some((deadline - itinerary.end).num_minutes()),
            });
            for (leg_index, leg) in itinerary.legs.iter().enumerate() {
                features.push(dto::TrasaFeaturesItem {
                    r#type: dto::TrasaFeaturesItemType::Feature,
                    geometry: dto::GeoLinie {
                        r#type: dto::GeoLinieType::LineString,
                        coordinates: leg.coordinates.clone(),
                    },
                    properties: dto::TrasaFeaturesItemProperties {
                        spoj,
                        usek: leg_index as i64,
                        druh: leg.mode.clone(),
                        linka: leg.line.clone(),
                        od: leg.from.clone(),
                        r#do: leg.to.clone(),
                        odjezd: leg.start.format("%H:%M").to_string(),
                        prijezd: leg.end.format("%H:%M").to_string(),
                        chuze_m: if leg.mode == dto::TrasaFeaturesItemPropertiesDruh::WALK {
                            Some(leg.distance.round() as i64)
                        } else {
                            None
                        },
                    },
                });
            }
        }
        Ok(dto::Trasa {
            r#type: dto::TrasaType::FeatureCollection,
            features,
            spoje,
            meta: dto::TrasaMeta {
                scenar: Some("rano".into()),
                okno_prijezdu: Some(vec!["07:00".into(), "08:00".into()]),
                den: Some(self.date.to_string()),
            },
        })
    }
}
#[derive(Deserialize)]
struct OtpResponse {
    #[serde(default)]
    errors: Vec<serde_json::Value>,
    data: Option<OtpData>,
}
#[derive(Deserialize)]
struct OtpData {
    #[serde(rename = "planConnection")]
    plan: Option<OtpPlan>,
}
#[derive(Deserialize)]
struct OtpPlan {
    #[serde(rename = "routingErrors")]
    errors: Vec<RoutingError>,
    edges: Vec<Option<Edge>>,
}
#[derive(Deserialize)]
struct RoutingError {
    code: String,
}
#[derive(Deserialize)]
struct Edge {
    node: Option<OtpItinerary>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtpItinerary {
    start: DateTime<chrono::FixedOffset>,
    end: DateTime<chrono::FixedOffset>,
    duration: f64,
    number_of_transfers: i64,
    walk_distance: f64,
    legs: Vec<OtpLeg>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtpLeg {
    #[serde(deserialize_with = "deserialize_mode")]
    mode: dto::TrasaFeaturesItemPropertiesDruh,
    distance: f64,
    start: LegTime,
    end: LegTime,
    from: Place,
    to: Place,
    route: Option<RouteName>,
    leg_geometry: Geometry,
}
// OTP distinguishes intercity coaches; the public API groups both as BUS.
fn deserialize_mode<'de, D>(
    deserializer: D,
) -> Result<dto::TrasaFeaturesItemPropertiesDruh, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mode = String::deserialize(deserializer)?;
    if mode == "COACH" {
        Ok(dto::TrasaFeaturesItemPropertiesDruh::BUS)
    } else {
        dto::TrasaFeaturesItemPropertiesDruh::deserialize(serde::de::value::StringDeserializer::<
            D::Error,
        >::new(mode))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegTime {
    scheduled_time: DateTime<chrono::FixedOffset>,
    estimated: Option<Estimated>,
}
impl LegTime {
    fn time(&self) -> DateTime<chrono_tz::Tz> {
        self.estimated
            .as_ref()
            .map(|e| e.time)
            .unwrap_or(self.scheduled_time)
            .with_timezone(&Prague)
    }
}
#[derive(Deserialize)]
struct Estimated {
    time: DateTime<chrono::FixedOffset>,
}
#[derive(Deserialize)]
struct Place {
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteName {
    short_name: Option<String>,
    long_name: Option<String>,
}
#[derive(Deserialize)]
struct Geometry {
    points: String,
}
fn parse(response: OtpResponse, date: NaiveDate) -> Result<RouteSet, StubError> {
    if !response.errors.is_empty() {
        return Err(unavailable());
    }
    let plan = response.data.and_then(|d| d.plan).ok_or_else(unavailable)?;
    if plan.errors.iter().any(|e| {
        !matches!(
            e.code.as_str(),
            "LOCATION_NOT_FOUND"
                | "NO_DIRECT_MODE_CONNECTION"
                | "NO_STOPS_IN_RANGE"
                | "NO_TRANSIT_CONNECTION"
                | "NO_TRANSIT_CONNECTION_IN_SEARCH_WINDOW"
                | "OUTSIDE_BOUNDS"
                | "WALKING_BETTER_THAN_TRANSIT"
        )
    }) {
        return Err(unavailable());
    }
    let (start, end) = window(date)?;
    let mut itineraries = Vec::new();
    for i in plan.edges.into_iter().flatten().filter_map(|e| e.node) {
        if !i.duration.is_finite()
            || i.duration < 0.0
            || !i.walk_distance.is_finite()
            || i.walk_distance < 0.0
            || i.number_of_transfers < 0
            || i.end < i.start
        {
            return Err(unavailable());
        }
        let departure = i.start.with_timezone(&Prague);
        let arrival = i.end.with_timezone(&Prague);
        if departure.date_naive() != date || arrival < start || arrival >= end {
            continue;
        }
        if i.legs.is_empty() {
            return Err(unavailable());
        }
        let mut legs = Vec::new();
        let mut previous = departure;
        for l in i.legs {
            let leg_start = l.start.time();
            let leg_end = l.end.time();
            if !l.distance.is_finite()
                || l.distance < 0.0
                || leg_start < previous
                || leg_end < leg_start
                || leg_end > arrival
            {
                return Err(unavailable());
            }
            previous = leg_end;
            let line = l.route.and_then(|r| {
                r.short_name
                    .filter(|n| !n.is_empty())
                    .or(r.long_name.filter(|n| !n.is_empty()))
            });
            legs.push(Leg {
                mode: l.mode,
                distance: l.distance,
                start: leg_start,
                end: leg_end,
                from: l.from.name,
                to: l.to.name,
                line,
                coordinates: decode_polyline(&l.leg_geometry.points)?,
            });
        }
        itineraries.push(Itinerary {
            start: departure,
            end: arrival,
            duration_seconds: i.duration,
            transfers: i.number_of_transfers,
            walk_meters: i.walk_distance,
            legs,
        });
    }
    itineraries.sort_by(|a, b| {
        a.duration_seconds
            .total_cmp(&b.duration_seconds)
            .then_with(|| b.end.cmp(&a.end))
            .then_with(|| a.start.cmp(&b.start))
    });
    // Equal-duration candidates can interleave duplicates with distinct routes.
    let mut distinct: Vec<Itinerary> = Vec::new();
    for itinerary in itineraries {
        if distinct.iter().any(|previous| {
            previous.start == itinerary.start
                && previous.end == itinerary.end
                && previous
                    .legs
                    .iter()
                    .map(|l| (&l.mode, &l.line, &l.coordinates))
                    .eq(itinerary
                        .legs
                        .iter()
                        .map(|l| (&l.mode, &l.line, &l.coordinates)))
        }) {
            continue;
        }
        distinct.push(itinerary);
        if distinct.len() == 3 {
            break;
        }
    }
    let itineraries = distinct;
    Ok(RouteSet { date, itineraries })
}
fn decode_polyline(encoded: &str) -> Result<Vec<Vec<f64>>, StubError> {
    let mut bytes = encoded.bytes();
    let mut lat = 0i64;
    let mut lon = 0i64;
    let mut points = Vec::new();
    fn delta(bytes: &mut std::str::Bytes<'_>) -> Result<i64, StubError> {
        let mut value = 0i64;
        let mut shift = 0;
        loop {
            let byte = bytes
                .next()
                .filter(|b| (63..=126).contains(b))
                .ok_or_else(unavailable)?
                - 63;
            if shift > 55 {
                return Err(unavailable());
            }
            value |= i64::from(byte & 31) << shift;
            shift += 5;
            if byte < 32 {
                break;
            }
        }
        Ok(if value & 1 != 0 {
            !(value >> 1)
        } else {
            value >> 1
        })
    }
    while bytes.len() > 0 {
        lat = lat
            .checked_add(delta(&mut bytes)?)
            .ok_or_else(unavailable)?;
        lon = lon
            .checked_add(delta(&mut bytes)?)
            .ok_or_else(unavailable)?;
        let latitude = lat as f64 / 100_000.0;
        let longitude = lon as f64 / 100_000.0;
        if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
            return Err(unavailable());
        }
        points.push(vec![longitude, latitude]);
    }
    if points.len() < 2 {
        return Err(unavailable());
    }
    Ok(points)
}
