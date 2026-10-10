//! Regional accessibility from a consistent snapshot of the precomputed matrix.
use crate::{
    catalog::{database_error, internal_error, read},
    contract::StubError,
    db::DbPool,
    dto, models, requests, schema,
};
use diesel::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) async fn zsj(
    pool: DbPool,
    params: requests::ZsjQuery,
) -> Result<dto::MapaDosahu, StubError> {
    read(pool, move |connection| {
        let (areas, population, school_ids, travel_times) = connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run::<_, diesel::result::Error, _>(|connection| {
                use schema::{
                    data_demografie_zsj as demo, dojezdove_doby as times, nabidka_oboru as offers,
                    stredni_skoly as schools, zsj,
                };
                let areas = zsj::table
                    .order(zsj::kod)
                    .select((
                        models::Zsj::as_select(),
                        diesel::dsl::sql::<diesel::sql_types::Text>(
                            "ST_AsGeoJSON(boundary, 15, 0)",
                        ),
                    ))
                    .load::<(models::Zsj, String)>(connection)?;
                let population = demo::table
                    .filter(demo::rok.eq(2021))
                    .filter(demo::demo_skupina.eq("1300100014"))
                    .select((demo::kod_zsj, demo::populace))
                    .load::<(String, i32)>(connection)?;
                let form = match params.forma {
                    requests::Forma::Den => "den",
                    requests::Forma::Dal => "dal",
                };
                let mut query = schools::table.into_boxed();
                if let Some(ref code) = params.obor {
                    query = query.filter(
                        schools::redizo.eq_any(
                            offers::table
                                .filter(offers::kod_oboru.eq(&code.0))
                                .filter(offers::forma_studia.eq(form))
                                .select(offers::redizo),
                        ),
                    );
                }
                let school_ids = query
                    .order(schools::redizo)
                    .select(schools::redizo)
                    .load::<String>(connection)?;
                let slot = match params.scenar {
                    requests::Scenar::Rano => "07:00-08:00",
                };
                let travel_times = times::table
                    .filter(times::slot_prijezdu.eq(slot))
                    .filter(times::redizo.eq_any(&school_ids))
                    .select(models::DojezdovaDoba::as_select())
                    .load::<models::DojezdovaDoba>(connection)?;
                Ok((areas, population, school_ids, travel_times))
            })
            .map_err(database_error)?;
        let population: BTreeMap<_, _> = population.into_iter().collect();
        let mut times = BTreeMap::<String, Vec<models::DojezdovaDoba>>::new();
        for time in travel_times {
            times.entry(time.kod_zsj.clone()).or_default().push(time);
        }
        let limits: BTreeSet<_> = [30, 45, 60, i64::from(params.max_min)]
            .into_iter()
            .collect();
        let mut counts: BTreeMap<_, i64> = limits.into_iter().map(|limit| (limit, 0)).collect();
        let mut features = Vec::with_capacity(areas.len());
        for (area, geometry) in areas {
            let people = *population.get(&area.kod).ok_or_else(internal_error)?;
            let children = cohort(people)?;
            let rows = times.remove(&area.kod).unwrap_or_default();
            if rows.len() != school_ids.len() {
                return Err(internal_error());
            }
            if rows
                .iter()
                .any(|row| row.doba_jizdy.is_some_and(|v| !v.is_finite() || v < 0.0))
            {
                return Err(internal_error());
            }
            let nearest = rows
                .iter()
                .filter_map(|row| row.doba_jizdy.map(|minutes| (minutes, row.redizo.as_str())))
                .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
            let duration = nearest.map(|(minutes, _)| minutes);
            let reachable = duration.is_some_and(|minutes| minutes <= f32::from(params.max_min));
            for (limit, count) in &mut counts {
                if duration.is_some_and(|minutes| minutes <= *limit as f32) {
                    *count += 1;
                }
            }
            let band = match duration {
                None if school_ids.is_empty() => dto::Pasmo::BezSpojeni,
                None => dto::Pasmo::DataNedostupna,
                Some(_) if !reachable => dto::Pasmo::MimoDosah,
                Some(v) if v <= 30.0 => dto::Pasmo::Do30,
                Some(v) if v <= 45.0 => dto::Pasmo::Od30Do45,
                Some(v) if v <= 60.0 => dto::Pasmo::Od45Do60,
                Some(_) => dto::Pasmo::Nad60,
            };
            features.push(dto::MapaDosahuFeaturesItem {
                r#type: dto::MapaDosahuFeaturesItemType::Feature,
                geometry: serde_json::from_str(&geometry).map_err(|_| internal_error())?,
                properties: dto::PlochaDosahu {
                    kod: area.kod,
                    nazev: area.nazev,
                    uroven: dto::PlochaDosahuUroven::Zsj,
                    cas_min: duration.map(|minutes| minutes.round() as i64),
                    pasmo: band,
                    v_dosahu: reachable,
                    deti: children,
                    deti_v_dosahu: Some(if reachable { children } else { 0 }),
                    podil_deti_v_dosahu: Some(if reachable && children > 0 {
                        100.0
                    } else {
                        0.0
                    }),
                    nejblizsi_redizo: nearest.map(|(_, redizo)| redizo.to_owned()),
                },
            });
        }
        Ok(dto::MapaDosahu {
            r#type: dto::MapaDosahuType::FeatureCollection,
            meta: dto::MapaDosahuMeta {
                scenar: match params.scenar {
                    requests::Scenar::Rano => "rano",
                }
                .to_owned(),
                max_min: i64::from(params.max_min),
                obor: params.obor.map(|code| code.0),
                forma: Some(
                    match params.forma {
                        requests::Forma::Den => "den",
                        requests::Forma::Dal => "dal",
                    }
                    .to_owned(),
                ),
                uroven: Some(dto::MapaDosahuMetaUroven::Zsj),
                jednotek_celkem: Some(features.len() as i64),
                v_limitu: Some(
                    counts
                        .into_iter()
                        .map(|(limit_min, jednotek)| dto::VLimitu {
                            limit_min,
                            jednotek,
                        })
                        .collect(),
                ),
            },
            features,
        })
    })
    .await
}

/// Shared school-entry estimate for maps and program detail.
pub(crate) fn cohort(population: i32) -> Result<i64, StubError> {
    if population < 0 {
        return Err(internal_error());
    }
    Ok((i64::from(population) + 2) / 5)
}
