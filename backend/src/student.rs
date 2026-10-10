//! Student school lookup using precomputed ZSJ-to-school travel times.
use crate::{
    catalog::{database_error, internal_error, offering, read},
    contract::{StubError, api_error},
    db::DbPool,
    dto, models, requests, schema,
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
    /// Preserve missing rows and NULL durations as distinct cases. Do not turn
    /// them into zero-minute journeys or remove journeys above max_min here.
    pub travel_times: Vec<models::DojezdovaDoba>,
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
    use schema::{
        dojezdove_doby as times, nabidka_oboru as offers, obory, stredni_skoly as schools, zsj,
    };
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
    let travel_times = if let Some(ref origin) = origin {
        times::table
            .filter(times::kod_zsj.eq(&origin.kod))
            .filter(times::slot_prijezdu.eq(arrival_slot))
            .filter(times::redizo.eq_any(schools.iter().map(|school| &school.redizo)))
            .order(times::redizo)
            .select(models::DojezdovaDoba::as_select())
            .load(connection)?
    } else {
        Vec::new()
    };
    Ok(StudentSchoolData {
        params,
        arrival_slot,
        origin,
        schools,
        offerings,
        travel_times,
    })
}

/// Transform a database snapshot into the public response without further I/O.
pub fn build_response(inputs: StudentSchoolData) -> Result<dto::StudentSkoly, StubError> {
    let origin = inputs.origin.ok_or_else(|| {
        api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "mimo_uzemi",
            "Bod leží mimo známé základní sídelní jednotky Karlovarského kraje.",
        )
    })?;
    let times: BTreeMap<_, _> = inputs
        .travel_times
        .into_iter()
        .map(|time| (time.redizo, time.doba_jizdy))
        .collect();
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
        // A complete matrix contains a row even when no duration was obtained.
        // Missing rows signal incomplete database input rather than no connection.
        let duration = *times.get(&school.redizo).ok_or_else(internal_error)?;
        if duration.is_some_and(|value| !value.is_finite() || value < 0.0) {
            return Err(internal_error());
        }
        let reachable = duration.is_some_and(|minutes| minutes <= f32::from(inputs.params.max_min));
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
                spoj: dto::Spoj {
                    // NULL mixes unavailable routes and routing errors in the seed.
                    // Report unavailable data, without claiming a route cannot exist.
                    stav: if duration.is_some() {
                        dto::SpojStav::Ok
                    } else {
                        dto::SpojStav::DataNedostupna
                    },
                    cas_min: duration.map(|minutes| minutes.round() as i64),
                    odjezd: None,
                    prijezd: None,
                    prestupy: None,
                    chuze_m: None,
                    linky: None,
                },
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
            presnost: Some("Orientační doba z reprezentačního bodu ZSJ; nejde o přesnou trasu ze zadaného bodu.".into()),
        },
    })
}
