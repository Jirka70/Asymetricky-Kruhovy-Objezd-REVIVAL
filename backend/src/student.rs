//! Database school selection with live, shared OTP route calculations.
use crate::{
    catalog::{database_error, internal_error, offering, read},
    contract::{StubError, api_error},
    db::DbPool,
    dto, models, otp, requests, schema,
};
use axum::http::StatusCode;
use bigdecimal::ToPrimitive;
use diesel::{
    prelude::*,
    sql_types::{Bool, Double},
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Internal database data, not the public StudentSkoly response DTO.
#[derive(Debug)]
pub struct StudentSchoolData {
    /// Request parameters used to select the arrival slot and reachable schools.
    pub params: requests::StudentSkolyQuery,
    pub arrival_slot: &'static str,
    /// None means the supplied point is outside every known ZSJ polygon.
    pub origin: Option<models::Zsj>,
    pub schools: Vec<models::Skola>,
    pub offerings: Vec<(models::NabidkaOboru, models::Obor)>,
}

pub async fn load_school_data(
    pool: DbPool,
    params: requests::StudentSkolyQuery,
) -> Result<StudentSchoolData, StubError> {
    read(pool, move |connection| {
        connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run(|connection| load(connection, params))
            .map_err(database_error)
    })
    .await
}

fn load(
    connection: &mut PgConnection,
    params: requests::StudentSkolyQuery,
) -> QueryResult<StudentSchoolData> {
    use schema::{nabidka_oboru as offers, obory, stredni_skoly as schools, zsj};
    let arrival_slot = match params.scenar {
        requests::Scenar::Rano => "07:00-08:00",
    };
    // PostGIS points use longitude first. Covers includes polygon boundaries;
    // a shared boundary is resolved deterministically by the smallest ZSJ code.
    // Bind coordinates rather than interpolating them into SQL.
    let contains_point = diesel::dsl::sql::<Bool>("ST_Covers(boundary, ST_SetSRID(ST_MakePoint(")
        .bind::<Double, _>(params.lon)
        .sql(", ")
        .bind::<Double, _>(params.lat)
        .sql("), 4326))");
    let origin = zsj::table
        .filter(contains_point)
        .order(zsj::kod)
        .select(models::Zsj::as_select())
        .first::<models::Zsj>(connection)
        .optional()?;
    let form = match params.forma {
        requests::Forma::Den => "den",
        requests::Forma::Dal => "dal",
    };
    let mut offering_query = offers::table
        .inner_join(obory::table)
        .filter(offers::forma_studia.eq(form))
        .into_boxed();
    if let Some(ref code) = params.obor {
        offering_query = offering_query.filter(offers::kod_oboru.eq(&code.0));
    }
    let offerings = offering_query
        .order((offers::redizo, offers::kod_oboru, offers::id))
        .select((models::NabidkaOboru::as_select(), models::Obor::as_select()))
        .load::<(models::NabidkaOboru, models::Obor)>(connection)?;
    let mut school_query = schools::table.into_boxed();
    if params.obor.is_some() {
        let matching: BTreeSet<_> = offerings
            .iter()
            .map(|(offer, _)| offer.redizo.as_str())
            .collect();
        school_query = school_query.filter(schools::redizo.eq_any(matching));
    }
    // With no program filter, retain schools without offerings as well.
    let schools = school_query
        .order(schools::redizo)
        .select(models::Skola::as_select())
        .load::<models::Skola>(connection)?;
    Ok(StudentSchoolData {
        params,
        arrival_slot,
        origin,
        schools,
        offerings,
    })
}

/// Transform a database snapshot into the public response without further I/O.
pub fn build_response(
    inputs: StudentSchoolData,
    routes: BTreeMap<String, Arc<otp::RouteSet>>,
    date: chrono::NaiveDate,
) -> Result<dto::StudentSkoly, StubError> {
    let origin = inputs.origin.ok_or_else(|| {
        api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "mimo_uzemi",
            "Bod leží mimo známé základní sídelní jednotky Karlovarského kraje.",
        )
    })?;
    let mut offerings = BTreeMap::<String, Vec<dto::Nabidka>>::new();
    for (offer, program) in inputs.offerings {
        offerings
            .entry(offer.redizo.clone())
            .or_default()
            .push(offering(offer, program)?);
    }
    let mut rows = Vec::new();
    let mut reachable_count = 0;
    let mut unreachable_count = 0;
    for school in inputs.schools {
        let route = routes.get(&school.redizo).ok_or_else(internal_error)?;
        let duration = route.itineraries.first().map(|i| i.duration_seconds / 60.0);
        let reachable = duration
            .is_some_and(|minutes| minutes <= requests::travel_limit(inputs.params.max_min));
        if reachable {
            reachable_count += 1;
        } else {
            unreachable_count += 1;
        }
        if inputs.params.obor.is_none() && !reachable {
            continue;
        }
        let nabidky = offerings.remove(&school.redizo).unwrap_or_default();
        let lat = school
            .lat
            .and_then(|value| value.to_f64())
            .filter(|v| v.is_finite())
            .ok_or_else(internal_error)?;
        let lon = school
            .lon
            .and_then(|value| value.to_f64())
            .filter(|v| v.is_finite())
            .ok_or_else(internal_error)?;
        rows.push((
            duration,
            dto::StudentSkolyDataItem {
                redizo: school.redizo,
                nazev: school.nazev.ok_or_else(internal_error)?,
                lat,
                lon,
                v_dosahu: reachable,
                spoj: route.summary(),
                nabidky,
            },
        ));
    }
    rows.sort_by(|(left_time, left), (right_time, right)| {
        match (left_time, right_time) {
            (Some(left), Some(right)) => left.total_cmp(right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| left.nazev.to_lowercase().cmp(&right.nazev.to_lowercase()))
        .then_with(|| left.redizo.cmp(&right.redizo))
    });
    Ok(dto::StudentSkoly {
        data: rows.into_iter().map(|(_, row)| row).collect(),
        meta: dto::StudentSkolyMeta {
            scenar: match inputs.params.scenar { requests::Scenar::Rano => "rano".into() },
            max_min: i64::from(inputs.params.max_min),
            zsj: Some(origin.kod),
            v_dosahu: Some(reachable_count),
            mimo_dosah: Some(unreachable_count),
            presnost: Some("Spojení OpenTripPlanner ze zadaného bodu; nejkratší z vrácených spojů s příjezdem 07:00–08:00 Europe/Prague.".into()),
            den: Some(date.to_string()),
        },
    })
}

fn outside() -> StubError {
    api_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "mimo_uzemi",
        "Bod leží mimo známé základní sídelní jednotky Karlovarského kraje.",
    )
}
fn coordinates(school: &models::Skola) -> Result<(f64, f64), StubError> {
    let lat = school
        .lat
        .as_ref()
        .and_then(|v| v.to_f64())
        .filter(|v| v.is_finite() && (-90.0..=90.0).contains(v))
        .ok_or_else(internal_error)?;
    let lon = school
        .lon
        .as_ref()
        .and_then(|v| v.to_f64())
        .filter(|v| v.is_finite() && (-180.0..=180.0).contains(v))
        .ok_or_else(internal_error)?;
    Ok((lat, lon))
}
pub async fn schools(
    pool: DbPool,
    params: requests::StudentSkolyQuery,
    client: otp::Client,
) -> Result<dto::StudentSkoly, StubError> {
    let origin = (params.lat, params.lon);
    let inputs = load_school_data(pool, params).await?;
    if inputs.origin.is_none() {
        return Err(outside());
    }
    let targets = inputs
        .schools
        .iter()
        .map(|s| {
            if s.nazev.is_none() {
                return Err(internal_error());
            }
            Ok((s.redizo.clone(), coordinates(s)?))
        })
        .collect::<Result<Vec<_>, StubError>>()?;
    let date = client.service_date();
    let mut tasks = tokio::task::JoinSet::new();
    for (redizo, destination) in targets {
        let client = client.clone();
        tasks.spawn(async move {
            client
                .routes(origin, destination, date)
                .await
                .map(|routes| (redizo, routes))
        });
    }
    let mut routes = BTreeMap::new();
    while let Some(result) = tasks.join_next().await {
        let (school, route) = result.map_err(|_| internal_error())??;
        routes.insert(school, route);
    }
    build_response(inputs, routes, date)
}
pub async fn route(
    pool: DbPool,
    params: requests::StudentTrasaQuery,
    client: otp::Client,
) -> Result<dto::Trasa, StubError> {
    let origin = (params.lat, params.lon);
    let destination = read(pool, move |connection| {
        connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run::<_, StubError, _>(|connection| {
                use schema::{stredni_skoly, zsj};
                let school = stredni_skoly::table
                    .find(&params.redizo.0)
                    .select(models::Skola::as_select())
                    .first::<models::Skola>(connection)
                    .optional()?
                    .ok_or_else(|| {
                        api_error(
                            StatusCode::NOT_FOUND,
                            "skola_nenalezena",
                            "Škola nebyla nalezena.",
                        )
                    })?;
                let contains =
                    diesel::dsl::sql::<Bool>("ST_Covers(boundary, ST_SetSRID(ST_MakePoint(")
                        .bind::<Double, _>(params.lon)
                        .sql(", ")
                        .bind::<Double, _>(params.lat)
                        .sql("), 4326))");
                if !diesel::select(diesel::dsl::exists(zsj::table.filter(contains)))
                    .get_result::<bool>(connection)?
                {
                    return Err(outside());
                }
                coordinates(&school)
            })
    })
    .await?;
    client
        .routes(origin, destination, client.service_date())
        .await?
        .geojson()
}
