//! School detail and travel-limit catchments from one read-only snapshot.
use crate::{
    catalog::{internal_error, read},
    contract::{StubError, api_error},
    db::DbPool,
    dto, models, requests, schema,
};
use axum::http::StatusCode;
use bigdecimal::ToPrimitive;
use diesel::{
    prelude::*,
    sql_types::{Float, Integer, Nullable, Text},
};
use std::collections::BTreeMap;

pub(crate) async fn get(
    pool: DbPool,
    redizo: String,
    params: requests::SkolaQuery,
) -> Result<dto::SkolaDetail, StubError> {
    read(pool, move |connection| {
        connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run::<_, StubError, _>(|connection| {
                let school = schema::stredni_skoly::table
                    .find(&redizo)
                    .select(models::Skola::as_select())
                    .first::<models::Skola>(connection)
                    .optional()
                    .map_err(|error| {
                        tracing::error!(%error, "Cannot load school");
                        internal_error()
                    })?
                    .ok_or_else(|| {
                        api_error(
                            StatusCode::NOT_FOUND,
                            "skola_nenalezena",
                            "Škola nebyla nalezena.",
                        )
                    })?;

                let offers = schema::nabidka_oboru::table
                    .inner_join(schema::obory::table)
                    .filter(schema::nabidka_oboru::redizo.eq(&redizo))
                    .select((models::NabidkaOboru::as_select(), models::Obor::as_select()))
                    .load::<(models::NabidkaOboru, models::Obor)>(connection)
                    .map_err(|error| {
                        tracing::error!(%error, "Cannot load school offerings");
                        internal_error()
                    })?;

                let nabidky = offers
                    .into_iter()
                    .map(|(offer, obor)| crate::catalog::offering(offer, obor))
                    .collect::<Result<Vec<_>, StubError>>()?;

                // These fields are nullable in the DB but required by the API.
                let nazev = school.nazev.ok_or_else(internal_error)?;
                let lat = school
                    .lat
                    .and_then(|value| value.to_f64())
                    .filter(|value| value.is_finite())
                    .ok_or_else(internal_error)?;
                let lon = school
                    .lon
                    .and_then(|value| value.to_f64())
                    .filter(|value| value.is_finite())
                    .ok_or_else(internal_error)?;

                Ok::<_, StubError>(dto::SkolaDetail {
                    redizo: school.redizo,
                    nazev,
                    adresa: school.adresa,
                    web: school.web,
                    lat,
                    lon,
                    nabidky,
                    spadovost: catchment(connection, &redizo, &params)?,
                    meta: Some(dto::MetaDosah {
                        scenar: "rano".into(),
                        max_min: i64::from(params.max_min),
                    }),
                })
            })
    })
    .await
}
#[derive(QueryableByName)]
struct CatchmentRow {
    #[diesel(sql_type=Nullable<Text>)]
    municipality: Option<String>,
    #[diesel(sql_type=Nullable<Text>)]
    name: Option<String>,
    #[diesel(sql_type=Nullable<Integer>)]
    population: Option<i32>,
    #[diesel(sql_type=Nullable<Text>)]
    time_code: Option<String>,
    #[diesel(sql_type=Nullable<Float>)]
    minutes: Option<f32>,
}
fn catchment(
    connection: &mut PgConnection,
    redizo: &str,
    params: &requests::SkolaQuery,
) -> Result<dto::SkolaDetailSpadovost, StubError> {
    let slot = match params.scenar {
        requests::Scenar::Rano => "07:00-08:00",
    };
    let rows=diesel::sql_query(r#"SELECT z.kod_obce AS municipality,o.nazev_obce AS name,d.populace AS population,t.kod_zsj AS time_code,t.doba_jizdy AS minutes
        FROM "ZSJ" z LEFT JOIN "SIMULATION_OBCE" o USING(kod_obce)
        LEFT JOIN "DATA_DEMOGRAFIE_ZSJ" d ON d.kod_zsj=z.kod AND d.rok=2021 AND d.demo_skupina='1300100014'
        LEFT JOIN "DOJEZDOVE_DOBY" t ON t.kod_zsj=z.kod AND t.redizo=$1 AND t.slot_prijezdu=$2 ORDER BY z.kod"#)
        .bind::<Text,_>(redizo).bind::<Text,_>(slot).load::<CatchmentRow>(connection)?;
    let mut municipalities = BTreeMap::<String, (String, i64, f32)>::new();
    let mut total = 0;
    for row in rows {
        let children = crate::reachability::cohort(row.population.ok_or_else(internal_error)?)?;
        if row.time_code.is_none() || row.minutes.is_some_and(|m| !m.is_finite() || m < 0.0) {
            return Err(internal_error());
        }
        let Some(minutes) = row.minutes.filter(|m| *m <= f32::from(params.max_min)) else {
            continue;
        };
        let code = row.municipality.ok_or_else(internal_error)?;
        let name = row.name.ok_or_else(internal_error)?;
        let entry = municipalities.entry(code).or_insert((name, 0, minutes));
        entry.1 += children;
        entry.2 = entry.2.min(minutes);
        total += children;
    }
    Ok(dto::SkolaDetailSpadovost {
        deti_v_dosahu: Some(total),
        obce: Some(
            municipalities
                .into_iter()
                .map(
                    |(code, (name, children, time))| dto::SkolaDetailSpadovostObceItem {
                        kod_obce: Some(code),
                        nazev: Some(name),
                        deti: Some(children),
                        nejkratsi_cas_min: Some(time.round() as i64),
                    },
                )
                .collect(),
        ),
    })
}
