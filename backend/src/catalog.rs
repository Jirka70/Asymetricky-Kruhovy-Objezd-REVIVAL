//! Read-only catalog queries. No OTP, travel-time, or reachability calculations.
use crate::{
    contract::{StubError, api_error},
    db::DbPool,
    dto, models, requests, schema,
    types::Vhodnost,
};
use axum::http::StatusCode;
use bigdecimal::ToPrimitive;
use diesel::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn internal_error() -> StubError {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "interni_chyba",
        "Nepodařilo se načíst data.",
    )
}

pub(crate) async fn read<T: Send + 'static>(
    pool: DbPool,
    operation: impl FnOnce(&mut PgConnection) -> Result<T, StubError> + Send + 'static,
) -> Result<T, StubError> {
    tokio::task::spawn_blocking(move || {
        let mut connection = pool.get().map_err(|error| {
            tracing::error!(%error, "Cannot obtain database connection");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "databaze_nedostupna",
                "Databáze je dočasně nedostupná.",
            )
        })?;
        operation(&mut connection)
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "Catalog query task failed");
        internal_error()
    })?
}

pub(crate) fn database_error(error: diesel::result::Error) -> StubError {
    tracing::error!(%error, "Catalog database query failed");
    internal_error()
}

pub(crate) async fn zsj(pool: DbPool) -> Result<Vec<dto::ZsjZaznam>, StubError> {
    read(pool, |connection| {
        use schema::zsj;
        let rows = zsj::table
            .order(zsj::kod)
            .select((
                models::Zsj::as_select(),
                // Suppress CRS metadata and retain polygon precision in WGS 84.
                diesel::dsl::sql::<diesel::sql_types::Text>("ST_AsGeoJSON(boundary, 15, 0)"),
            ))
            .load::<(models::Zsj, String)>(connection)
            .map_err(database_error)?;
        rows.into_iter()
            .map(|(row, boundary)| {
                if !row.lat.is_finite() || !row.lon.is_finite() {
                    return Err(internal_error());
                }
                let boundary =
                    serde_json::from_str::<dto::ZsjBoundary>(&boundary).map_err(|error| {
                        tracing::error!(%error, kod = %row.kod, "Invalid ZSJ polygon");
                        internal_error()
                    })?;
                Ok(dto::ZsjZaznam {
                    kod: row.kod,
                    nazev: row.nazev,
                    lat: row.lat,
                    lon: row.lon,
                    boundary,
                    kod_obce: row.kod_obce,
                })
            })
            .collect()
    })
    .await
}

pub(crate) async fn schools(
    pool: DbPool,
    params: requests::SkolyQuery,
) -> Result<dto::SkolyFeatureCollection, StubError> {
    read(pool, move |connection| {
        use schema::{nabidka_oboru as offers, stredni_skoly as schools};
        let schools = schools::table
            .order(schools::redizo)
            .select(models::Skola::as_select())
            .load(connection)
            .map_err(database_error)?;
        let forma = match params.forma {
            requests::Forma::Den => "den",
            requests::Forma::Dal => "dal",
        };
        let mut query = offers::table
            .filter(offers::forma_studia.eq(forma))
            .into_boxed();
        if let Some(ref obor) = params.obor {
            query = query.filter(offers::kod_oboru.eq(&obor.0));
        }
        if let Some(ref stupne) = params.stupen {
            let codes: Vec<String> = stupne
                .0
                .iter()
                .map(|stupen| {
                    serde_json::to_value(stupen)
                        .expect("Stupen serialization")
                        .as_str()
                        .unwrap()
                        .to_owned()
                })
                .collect();
            // KKOV codes have the education level in position 7: 65-51-H/01.
            query = query.filter(
                diesel::dsl::sql::<diesel::sql_types::Text>("substring(kod_oboru from 7 for 1)")
                    .eq_any(codes),
            );
        }
        let offers = query
            .select(models::NabidkaOboru::as_select())
            .load::<models::NabidkaOboru>(connection)
            .map_err(database_error)?;
        // One response feature per school, with totals over the matching offerings.
        let mut totals = BTreeMap::<String, (i64, i64, i64)>::new();
        for offer in offers {
            let total = totals.entry(offer.redizo).or_default();
            total.0 += 1;
            total.1 += i64::from(offer.pocet_prijimanych);
            total.2 += i64::from(offer.loni_pocet_prihlasek);
        }
        let filtered = params.obor.is_some() || params.stupen.is_some();
        let mut features = Vec::new();
        for school in schools {
            let (count, capacity, applications) = totals.remove(&school.redizo).unwrap_or_default();
            // Without obor/stupen, retain schools with no matching offerings.
            if filtered && count == 0 {
                continue;
            }
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
            features.push(dto::SkolyFeatureCollectionFeaturesItem {
                r#type: dto::SkolyFeatureCollectionFeaturesItemType::Feature,
                geometry: dto::GeoPoint {
                    r#type: dto::GeoPointType::Point,
                    coordinates: vec![lon, lat],
                },
                properties: dto::SkolyFeatureCollectionFeaturesItemProperties {
                    redizo: school.redizo,
                    nazev: school.nazev.ok_or_else(internal_error)?,
                    obec: None,
                    pocet_nabidek: count,
                    kapacita: Some(capacity),
                    prihlasky: Some(applications),
                    index_pretlaku: None,
                },
            });
        }
        Ok(dto::SkolyFeatureCollection {
            r#type: dto::SkolyFeatureCollectionType::FeatureCollection,
            features,
        })
    })
    .await
}

pub(crate) async fn employers(
    pool: DbPool,
    kod: String,
    params: requests::OborZamestnavateleQuery,
) -> Result<dto::ZamestnavateleOboru, StubError> {
    read(pool, move |connection| {
        use schema::{
            obor_profese as mapping, obory, poptavka_profesi as demand, profesni_skupiny as groups,
            zamestnavatele as workplaces,
        };
        let exists = diesel::select(diesel::dsl::exists(
            obory::table.filter(obory::kod.eq(&kod)),
        ))
        .get_result::<bool>(connection)
        .map_err(database_error)?;
        if !exists {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "obor_nenalezen",
                "Obor nebyl nalezen.",
            ));
        }
        let mapped = diesel::select(diesel::dsl::exists(
            mapping::table.filter(mapping::kod_oboru.eq(&kod)),
        ))
        .get_result::<bool>(connection)
        .map_err(database_error)?;
        let suitability = params.vhodnost as i16;
        let group_rows = mapping::table
            .inner_join(groups::table)
            .filter(mapping::kod_oboru.eq(&kod))
            .filter(mapping::vhodnost.le(suitability))
            .order(mapping::cz_isco3)
            .select((
                models::OborProfese::as_select(),
                models::ProfesniSkupina::as_select(),
            ))
            .load::<(models::OborProfese, models::ProfesniSkupina)>(connection)
            .map_err(database_error)?;
        let skupiny = group_rows
            .into_iter()
            .map(|(mapping, group)| {
                Ok(dto::ZamestnavateleOboruMetaSkupinyItem {
                    cz_isco3: Some(group.cz_isco3),
                    nazev: Some(group.nazev),
                    vhodnost: Some(vhodnost(mapping.vhodnost)?),
                })
            })
            .collect::<Result<Vec<_>, StubError>>()?;
        let mut query = demand::table
            .inner_join(workplaces::table)
            .inner_join(groups::table)
            .inner_join(mapping::table.on(mapping::cz_isco3.eq(demand::cz_isco3)))
            .filter(mapping::kod_oboru.eq(&kod))
            .filter(mapping::vhodnost.le(suitability))
            .filter(demand::pocet_mist.gt(0))
            .into_boxed();
        if params.jen_ss {
            query = query.filter(demand::min_vzdelani.ne_all([
                "vyssOdbor",
                "bakal",
                "vysoka",
                "doktor",
            ]));
        }
        let rows = query
            .order((workplaces::id, demand::cz_isco3, demand::min_vzdelani))
            .select((
                models::Zamestnavatel::as_select(),
                models::PoptavkaProfesi::as_select(),
                models::ProfesniSkupina::as_select(),
                models::OborProfese::as_select(),
            ))
            .load::<(
                models::Zamestnavatel,
                models::PoptavkaProfesi,
                models::ProfesniSkupina,
                models::OborProfese,
            )>(connection)
            .map_err(database_error)?;
        // Aggregate education categories into professions, then into workplaces.
        let mut entries = BTreeMap::<i64, Workplace>::new();
        let mut imported = None;
        for (workplace, demand, group, mapping) in rows {
            imported = Some(imported.map_or(
                demand.importovano_at,
                |previous: chrono::DateTime<chrono::Utc>| previous.max(demand.importovano_at),
            ));
            let entry = entries.entry(workplace.id).or_insert_with(|| Workplace {
                workplace,
                count: 0,
                professions: BTreeMap::new(),
            });
            entry.count += i64::from(demand.pocet_mist);
            let profession = entry.professions.entry(group.cz_isco3.clone()).or_insert(
                dto::ZamestnavateleOboruFeaturesItemPropertiesProfeseItem {
                    cz_isco3: Some(group.cz_isco3),
                    nazev: Some(group.nazev),
                    vhodnost: Some(vhodnost(mapping.vhodnost)?),
                    pocet_mist: Some(0),
                },
            );
            *profession.pocet_mist.as_mut().unwrap() += i64::from(demand.pocet_mist);
        }
        let company_count = entries
            .values()
            .map(|entry| &entry.workplace.ico)
            .collect::<BTreeSet<_>>()
            .len() as i64;
        let workplace_count = entries.len() as i64;
        let job_count = entries.values().map(|entry| entry.count).sum();
        let mut missing = BTreeMap::<String, (i64, i64)>::new();
        let mut features = Vec::new();
        for entry in entries.into_values() {
            let workplace = entry.workplace;
            match (workplace.lon, workplace.lat) {
                (Some(lon), Some(lat)) if lon.is_finite() && lat.is_finite() => {
                    features.push(dto::ZamestnavateleOboruFeaturesItem {
                        r#type: dto::ZamestnavateleOboruFeaturesItemType::Feature,
                        geometry: dto::GeoPoint {
                            r#type: dto::GeoPointType::Point,
                            coordinates: vec![lon, lat],
                        },
                        properties: dto::ZamestnavateleOboruFeaturesItemProperties {
                            id: workplace.id.to_string(),
                            ico: workplace.ico,
                            nazev: workplace.nazev,
                            kod_obce: workplace.kod_obce,
                            pocet_mist: entry.count,
                            profese: entry.professions.into_values().collect(),
                        },
                    })
                }
                _ => {
                    let total = missing.entry(workplace.kod_obce).or_default();
                    total.0 += 1;
                    total.1 += entry.count;
                }
            }
        }
        Ok(dto::ZamestnavateleOboru {
            r#type: dto::ZamestnavateleOboruType::FeatureCollection,
            features,
            meta: dto::ZamestnavateleOboruMeta {
                obor: kod,
                mapovani: mapped,
                existuje: mapped.then_some(workplace_count > 0),
                zamestnavatelu: mapped.then_some(company_count),
                pracovist: mapped.then_some(workplace_count),
                pocet_mist: mapped.then_some(job_count),
                bez_souradnic: Some(
                    missing
                        .into_iter()
                        .map(
                            |(kod, (count, jobs))| dto::ZamestnavateleOboruMetaBezSouradnicItem {
                                kod_obce: Some(kod),
                                pracovist: Some(count),
                                pocet_mist: Some(jobs),
                            },
                        )
                        .collect(),
                ),
                skupiny: Some(skupiny),
                importovano: imported.map(|date| date.to_rfc3339()),
            },
        })
    })
    .await
}

struct Workplace {
    workplace: models::Zamestnavatel,
    count: i64,
    professions: BTreeMap<String, dto::ZamestnavateleOboruFeaturesItemPropertiesProfeseItem>,
}
pub(crate) fn vhodnost(value: i16) -> Result<Vhodnost, StubError> {
    match value {
        1 => Ok(Vhodnost::Nejvhodnejsi),
        2 => Ok(Vhodnost::Vhodne),
        _ => Err(internal_error()),
    }
}

/// Shared conversion for school details and the student school response.
pub(crate) fn offering(
    offer: models::NabidkaOboru,
    program: models::Obor,
) -> Result<dto::Nabidka, StubError> {
    let forma = match offer.forma_studia.as_str() {
        "den" => dto::NabidkaForma::Den,
        "dal" => dto::NabidkaForma::Dal,
        _ => return Err(internal_error()),
    };
    Ok(dto::Nabidka {
        kod_oboru: program.kod,
        nazev_oboru: program.nazev,
        zamereni: offer.display_name,
        stupen: None,
        forma,
        delka_let: Some(i64::from(offer.delka_studia)),
        kapacita: i64::from(offer.pocet_prijimanych),
        prihlasky: i64::from(offer.loni_pocet_prihlasek),
        prihlasky_na_misto: (offer.pocet_prijimanych > 0)
            .then(|| f64::from(offer.loni_pocet_prihlasek) / f64::from(offer.pocet_prijimanych)),
        index_pretlaku: None,
    })
}
