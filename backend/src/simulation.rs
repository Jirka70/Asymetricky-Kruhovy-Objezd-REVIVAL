//! Read-only simulation of additional daily-study capacity, following core_job_spec.
use crate::{
    catalog::{database_error, internal_error, read},
    contract::{SimulaceResponse, StubError, api_error},
    db::DbPool,
    dto, models, programs, requests, schema,
};
use axum::http::StatusCode;
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer, Nullable, Text},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub redizo: String,
    pub kod_oboru: String,
    pub kapacita: i64,
    pub prihlasky: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub code: String,
    pub name: String,
    pub municipality: Option<String>,
    pub municipality_name: Option<String>,
    pub orp: Option<String>,
    pub orp_name: Option<String>,
    pub children: f64,
    pub population: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub program_exists: bool,
    pub schools: BTreeMap<String, String>,
    pub offers: Vec<Offer>,
    pub areas: Vec<Area>,
    pub times: Vec<(String, String, Option<f64>)>,
}
#[derive(Debug, Clone)]
struct Row {
    area: Area,
    before: Option<f64>,
    after: Option<f64>,
    target: Option<f64>,
    school_after: Option<String>,
    improved: bool,
    new_reach: bool,
    demand: f64,
}
pub struct Calculation {
    rows: Vec<Row>,
    pub summary: Option<dto::SimulaceSouhrn>,
    pub meta: dto::SimulaceMeta,
}

const UTILIZATION_THRESHOLD: f64 = 0.5;

fn round(value: f64, digits: i32) -> f64 {
    // Decimal formatting rounds the original binary float, matching Python round.
    format!("{value:.precision$}", precision = digits as usize)
        .parse()
        .expect("formatted finite number")
}
fn band(time: Option<f64>, limit: u16) -> dto::Pasmo {
    match time {
        None => dto::Pasmo::BezSpojeni,
        Some(t) if t > f64::from(limit) => dto::Pasmo::MimoDosah,
        Some(t) if t <= 30.0 => dto::Pasmo::Do30,
        Some(t) if t <= 45.0 => dto::Pasmo::Od30Do45,
        Some(t) if t <= 60.0 => dto::Pasmo::Od45Do60,
        Some(_) => dto::Pasmo::Nad60,
    }
}
fn in_limit(time: Option<f64>, limit: f64) -> bool {
    time.is_some_and(|t| t <= limit)
}
fn level(level: requests::Uroven) -> dto::SimulaceMetaUroven {
    match level {
        requests::Uroven::Zsj => dto::SimulaceMetaUroven::Zsj,
        requests::Uroven::Obec => dto::SimulaceMetaUroven::Obec,
        requests::Uroven::Orp => dto::SimulaceMetaUroven::Orp,
    }
}

/// Pure calculations from an already loaded snapshot; no SQL or live routing.
pub fn calculate(input: Input, params: &requests::SimulaceQuery) -> Result<Calculation, StubError> {
    if !(1..=300).contains(&params.kapacita) {
        return Err(StubError::invalid("kapacita"));
    }
    if !input.schools.contains_key(&params.redizo.0) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "skola_nenalezena",
            "Škola nebyla nalezena.",
        ));
    }
    if !input.program_exists {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "obor_nenalezen",
            "Obor nebyl nalezen.",
        ));
    }
    let mut capacities = BTreeMap::<String, i64>::new();
    let mut applications = 0;
    let mut all_applications = 0;
    for offer in &input.offers {
        if offer.kapacita < 0 || offer.prihlasky < 0 {
            return Err(internal_error());
        }
        all_applications += offer.prihlasky;
        if offer.kod_oboru == params.obor.0 {
            if !input.schools.contains_key(&offer.redizo) {
                return Err(internal_error());
            }
            *capacities.entry(offer.redizo.clone()).or_default() += offer.kapacita;
            applications += offer.prihlasky;
        }
    }
    if capacities.is_empty() {
        return Err(api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "obor_bez_nabidky",
            "Obor nemá denní nabídky v kraji; nelze odhadnout zájem.",
        ));
    }
    let existing = capacities.contains_key(&params.redizo.0);
    let regional_capacity = capacities.values().sum::<i64>();
    let pressure_ratio = programs::ratio(applications, regional_capacity);
    let interest = programs::ratio(applications, all_applications).unwrap_or(0.0);
    let mut meta = dto::SimulaceMeta {
        scenar: "rano".into(),
        max_min: i64::from(params.max_min),
        redizo: Some(params.redizo.0.clone()),
        obor: Some(params.obor.0.clone()),
        uroven: Some(level(params.uroven)),
        podil_zajmu: Some(round(interest, 4)),
        skola_obor_uz_uci: Some(existing),
        prihlasky_na_misto_kraj: pressure_ratio.map(|v| round(v, 2)),
        index_pretlaku: pressure_ratio
            .map(|v| round(v / programs::REGIONAL_APPLICATIONS_PER_PLACE, 2)),
        duvod: None,
    };
    if existing && pressure_ratio.is_some_and(|v| v <= programs::REGIONAL_APPLICATIONS_PER_PLACE) {
        meta.duvod = Some("kapacita_staci".into());
        return Ok(Calculation {
            rows: vec![],
            summary: None,
            meta,
        });
    }
    let mut times = BTreeMap::<String, BTreeMap<String, Option<f64>>>::new();
    for (area, school, time) in input.times {
        if time.is_some_and(|v| !v.is_finite() || v < 0.0) {
            return Err(internal_error());
        }
        if times
            .entry(area)
            .or_default()
            .insert(school, time)
            .is_some()
        {
            return Err(internal_error());
        }
    }
    let mut after_capacity = capacities.clone();
    *after_capacity.entry(params.redizo.0.clone()).or_default() += i64::from(params.kapacita);
    let mut before_catchment = BTreeMap::<String, f64>::new();
    let mut after_catchment = BTreeMap::<String, f64>::new();
    let mut rows = Vec::new();
    let mut before_assignments = Vec::new();
    let mut seen = BTreeSet::new();
    for area in input.areas {
        if !seen.insert(area.code.clone())
            || !area.children.is_finite()
            || area.children < 0.0
            || !area.population.is_finite()
            || area.population < 0.0
        {
            return Err(internal_error());
        }
        let area_times = times.get(&area.code).ok_or_else(internal_error)?;
        for school in after_capacity.keys() {
            if !area_times.contains_key(school) {
                return Err(internal_error());
            }
        }
        let nearest = |schools: &BTreeMap<String, i64>| {
            schools
                .keys()
                .filter_map(|s| area_times[s].map(|t| (t, s.clone())))
                .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        };
        let before = nearest(&capacities);
        let after = nearest(&after_capacity);
        let t = area_times[&params.redizo.0];
        let demand = area.children * interest;
        let before_time = before.as_ref().map(|v| v.0);
        let after_time = after.as_ref().map(|v| v.0);
        if let Some((t, s)) = &before {
            if *t <= f64::from(params.max_min) {
                *before_catchment.entry(s.clone()).or_default() += demand;
            }
        }
        if let Some((t, s)) = &after {
            if *t <= f64::from(params.max_min) {
                *after_catchment.entry(s.clone()).or_default() += demand;
            }
        }
        before_assignments.push(before.map(|v| v.1));
        rows.push(Row {
            improved: t.is_some_and(|t| before_time.is_none_or(|b| t < b)),
            new_reach: !in_limit(before_time, f64::from(params.max_min))
                && in_limit(after_time, f64::from(params.max_min)),
            area,
            before: before_time,
            after: after_time,
            target: t,
            school_after: after.map(|v| v.1),
            demand,
        });
    }
    let balance: BTreeMap<_, _> = capacities
        .iter()
        .map(|(s, c)| {
            (
                s.clone(),
                *c as f64 - before_catchment.get(s).copied().unwrap_or(0.0),
            )
        })
        .collect();
    let mut relief = 0.0;
    let mut transfers = 0.0;
    let mut new_demand = 0.0;
    for (row, previous) in rows.iter().zip(before_assignments) {
        if row.school_after.as_deref() != Some(params.redizo.0.as_str())
            || !in_limit(row.after, f64::from(params.max_min))
        {
            continue;
        }
        if !in_limit(row.before, f64::from(params.max_min)) {
            new_demand += row.demand;
        } else if previous.as_deref() != Some(params.redizo.0.as_str()) {
            if previous.as_ref().is_some_and(|s| balance[s] < 0.0) {
                relief += row.demand;
            } else {
                transfers += row.demand;
            }
        }
    }
    let utilization = (relief + new_demand) / f64::from(params.kapacita);
    let verdict = if utilization >= UTILIZATION_THRESHOLD {
        dto::SimulaceVerdikt::DobreMisto
    } else if transfers > relief {
        dto::SimulaceVerdikt::SpatneMisto
    } else {
        dto::SimulaceVerdikt::Neutralni
    };
    let deficit = balance
        .iter()
        .filter(|(s, b)| **b < 0.0 && *s != &params.redizo.0)
        .min_by(|a, b| a.1.total_cmp(b.1).then_with(|| a.0.cmp(b.0)))
        .map(|(s, b)| dto::SimulaceDeficit {
            redizo: s.clone(),
            nazev: input.schools[s].clone(),
            chybi_mist: round(-b, 1),
        });
    let potential = after_catchment
        .get(&params.redizo.0)
        .copied()
        .unwrap_or(0.0);
    let summary = dto::SimulaceSouhrn {
        jednotek_celkem: None,
        v_limitu: None,
        zlepsenych_jednotek: None,
        prumerne_zkraceni_min: None,
        potencialni_uchazeci: Some(round(potential, 1)),
        blize_ke_skole: Some(round(potential, 1)),
        kapacita: Some(i64::from(params.kapacita)),
        uchazecu_na_misto: Some(round(potential / f64::from(params.kapacita), 2)),
        bilance_skol: Some(
            after_capacity
                .into_iter()
                .map(|(s, c)| dto::SimulaceBilanceSkoly {
                    nazev: input.schools[&s].clone(),
                    kapacita: c,
                    spad_pred: round(before_catchment.get(&s).copied().unwrap_or(0.0), 1),
                    spad_po: round(after_catchment.get(&s).copied().unwrap_or(0.0), 1),
                    redizo: s,
                })
                .collect(),
        ),
        odlehceni: Some(round(relief, 1)),
        pretazeni: Some(round(transfers, 1)),
        novi_v_dosahu: Some(round(new_demand, 1)),
        vyuziti: Some(round(utilization, 2)),
        verdikt: Some(verdict),
        nejvetsi_deficit: deficit,
    };
    Ok(Calculation {
        rows,
        summary: Some(summary),
        meta,
    })
}

fn weighted(rows: &[&Row], time: impl Fn(&Row) -> Option<f64>) -> Option<f64> {
    let mut sum = 0.0;
    let mut weight = 0.0;
    for row in rows {
        if let Some(value) = time(row) {
            sum += value * row.area.population;
            weight += row.area.population;
        }
    }
    (weight > 0.0).then(|| sum / weight)
}
struct OutputArea {
    code: String,
    value: dto::SimulacePlocha,
    improved: bool,
    before: Option<f64>,
    after: Option<f64>,
    children_reached: f64,
    school: Option<String>,
}
fn aggregate(rows: &[Row], params: &requests::SimulaceQuery) -> Result<Vec<OutputArea>, StubError> {
    let mut groups = BTreeMap::<String, Vec<&Row>>::new();
    for row in rows {
        let code = match params.uroven {
            requests::Uroven::Zsj => &row.area.code,
            requests::Uroven::Obec => row.area.municipality.as_ref().ok_or_else(internal_error)?,
            requests::Uroven::Orp => row.area.orp.as_ref().ok_or_else(internal_error)?,
        };
        groups.entry(code.clone()).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(code, rows)| {
            let first = rows[0];
            let zsj = params.uroven == requests::Uroven::Zsj;
            let name = match params.uroven {
                requests::Uroven::Zsj => first.area.name.clone(),
                requests::Uroven::Obec => first
                    .area
                    .municipality_name
                    .clone()
                    .ok_or_else(internal_error)?,
                requests::Uroven::Orp => first.area.orp_name.clone().ok_or_else(internal_error)?,
            };
            let before = if zsj {
                first.before
            } else {
                weighted(&rows, |r| r.before)
            };
            let after = if zsj {
                first.after
            } else {
                weighted(&rows, |r| r.after)
            };
            let target = if zsj {
                first.target
            } else {
                weighted(&rows, |r| r.target)
            };
            let improved = rows.iter().any(|r| r.improved);
            let improvement = if !improved {
                Some(0.0)
            } else {
                before.zip(after).map(|(b, a)| round(b - a, 2))
            };
            let children = rows.iter().map(|r| r.area.children).sum::<f64>();
            let children_reached = rows
                .iter()
                .filter(|r| in_limit(r.after, f64::from(params.max_min)))
                .map(|r| r.area.children)
                .sum::<f64>();
            Ok(OutputArea {
                code,
                improved,
                before,
                after,
                children_reached,
                school: if zsj {
                    first.school_after.clone()
                } else {
                    None
                },
                value: dto::SimulacePlocha {
                    nazev: name,
                    cas_ke_skole: target.map(|t| round(t, 2)),
                    cas_min_puvodni: before.map(|t| round(t, 2)),
                    cas_min: after.map(|t| round(t, 2)),
                    zlepseni_min: improvement,
                    pasmo: band(after, params.max_min),
                    pasmo_puvodni: Some(band(before, params.max_min)),
                    deti: round(children, 1),
                    potencialni_uchazeci: round(rows.iter().map(|r| r.demand).sum(), 2),
                    novy_dosah: rows.iter().any(|r| r.new_reach),
                },
            })
        })
        .collect()
}

impl Calculation {
    pub fn render(
        mut self,
        params: &requests::SimulaceQuery,
        geometries: BTreeMap<String, dto::GeoPlocha>,
    ) -> Result<SimulaceResponse, StubError> {
        let areas = aggregate(&self.rows, params)?;
        if let Some(summary) = &mut self.summary {
            summary.jednotek_celkem = Some(areas.len() as i64);
            summary.zlepsenych_jednotek = Some(areas.iter().filter(|a| a.improved).count() as i64);
            let improvements: Vec<_> = areas
                .iter()
                .filter(|a| a.improved)
                .filter_map(|a| a.before.zip(a.after).map(|(b, a)| b - a))
                .collect();
            summary.prumerne_zkraceni_min = Some(if improvements.is_empty() {
                0.0
            } else {
                round(
                    improvements.iter().sum::<f64>() / improvements.len() as f64,
                    1,
                )
            });
            let limits: BTreeSet<i64> = [30, 45, 60, i64::from(params.max_min)]
                .into_iter()
                .collect();
            summary.v_limitu = Some(
                limits
                    .into_iter()
                    .map(|l| dto::VLimituPredPo {
                        limit_min: l,
                        pred: areas
                            .iter()
                            .filter(|a| in_limit(a.before, l as f64))
                            .count() as i64,
                        po: areas.iter().filter(|a| in_limit(a.after, l as f64)).count() as i64,
                    })
                    .collect(),
            );
        }
        match params.format {
            requests::Format::Slovnik => Ok(SimulaceResponse::Slovnik(dto::Simulace {
                jednotky: areas
                    .into_iter()
                    .filter(|a| a.improved)
                    .map(|a| (a.code, a.value))
                    .collect(),
                souhrn: self.summary,
                meta: self.meta,
            })),
            requests::Format::Geojson => {
                let features = areas
                    .into_iter()
                    .map(|a| {
                        let p = a.value;
                        Ok(dto::SimulaceGeojsonFeaturesItem {
                            r#type: Some(dto::SimulaceGeojsonFeaturesItemType::Feature),
                            geometry: Some(
                                geometries
                                    .get(&a.code)
                                    .cloned()
                                    .ok_or_else(internal_error)?,
                            ),
                            properties: Some(dto::SimulaceGeojsonFeaturesItemProperties {
                                kod: a.code,
                                nazev: p.nazev,
                                uroven: match params.uroven {
                                    requests::Uroven::Zsj => {
                                        dto::SimulaceGeojsonFeaturesItemPropertiesUroven::Zsj
                                    }
                                    requests::Uroven::Obec => {
                                        dto::SimulaceGeojsonFeaturesItemPropertiesUroven::Obec
                                    }
                                    requests::Uroven::Orp => {
                                        dto::SimulaceGeojsonFeaturesItemPropertiesUroven::Orp
                                    }
                                },
                                cas_min: p.cas_min,
                                pasmo: p.pasmo,
                                v_dosahu: in_limit(a.after, f64::from(params.max_min)),
                                deti: p.deti,
                                deti_v_dosahu: Some(round(a.children_reached, 1)),
                                podil_deti_v_dosahu: Some(if p.deti > 0.0 {
                                    round(100.0 * a.children_reached / p.deti, 2)
                                } else {
                                    0.0
                                }),
                                nejblizsi_redizo: a.school,
                                cas_ke_skole: p.cas_ke_skole,
                                cas_min_puvodni: p.cas_min_puvodni,
                                zlepseni_min: p.zlepseni_min,
                                pasmo_puvodni: p.pasmo_puvodni,
                                potencialni_uchazeci: p.potencialni_uchazeci,
                                novy_dosah: p.novy_dosah,
                            }),
                        })
                    })
                    .collect::<Result<Vec<_>, StubError>>()?;
                Ok(SimulaceResponse::Geojson(dto::SimulaceGeojson {
                    r#type: dto::SimulaceGeojsonType::FeatureCollection,
                    features,
                    souhrn: self.summary,
                    meta: self.meta,
                }))
            }
        }
    }
}

#[derive(QueryableByName)]
struct AreaData {
    #[diesel(sql_type=Text)]
    code: String,
    #[diesel(sql_type=Text)]
    name: String,
    #[diesel(sql_type=Nullable<Text>)]
    municipality: Option<String>,
    #[diesel(sql_type=Nullable<Text>)]
    municipality_name: Option<String>,
    #[diesel(sql_type=Nullable<Text>)]
    orp: Option<String>,
    #[diesel(sql_type=Nullable<Text>)]
    orp_name: Option<String>,
    #[diesel(sql_type=Nullable<BigInt>)]
    population: Option<i64>,
    #[diesel(sql_type=Nullable<Integer>)]
    children: Option<i32>,
    #[diesel(sql_type=Nullable<BigInt>)]
    age_groups: Option<i64>,
}
#[derive(QueryableByName)]
struct GeometryData {
    #[diesel(sql_type=Text)]
    code: String,
    #[diesel(sql_type=Text)]
    geometry: String,
}

pub(crate) async fn get(
    pool: DbPool,
    params: requests::SimulaceQuery,
) -> Result<SimulaceResponse, StubError> {
    read(pool, move |connection| {
        connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run::<_, StubError, _>(|connection| load_and_render(connection, &params))
    })
    .await
}

fn load_and_render(
    connection: &mut PgConnection,
    params: &requests::SimulaceQuery,
) -> Result<SimulaceResponse, StubError> {
    use schema::{
        dojezdove_doby as times, nabidka_oboru as offers, obory, stredni_skoly as schools,
    };
    let school_rows = schools::table
        .select((schools::redizo, schools::nazev))
        .load::<(String, Option<String>)>(connection)
        .map_err(database_error)?;
    if !school_rows.iter().any(|(code, _)| code == &params.redizo.0) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "skola_nenalezena",
            "Škola nebyla nalezena.",
        ));
    }
    let schools = school_rows
        .into_iter()
        .map(|(s, n)| n.map(|n| (s, n)).ok_or_else(internal_error))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let program_exists = diesel::select(diesel::dsl::exists(obory::table.find(&params.obor.0)))
        .get_result(connection)
        .map_err(database_error)?;
    let offers = offers::table
        .filter(offers::forma_studia.eq("den"))
        .select(models::NabidkaOboru::as_select())
        .load::<models::NabidkaOboru>(connection)
        .map_err(database_error)?
        .into_iter()
        .map(|o| Offer {
            redizo: o.redizo,
            kod_oboru: o.kod_oboru,
            kapacita: i64::from(o.pocet_prijimanych),
            prihlasky: i64::from(o.loni_pocet_prihlasek),
        })
        .collect();
    let mut input = Input {
        program_exists,
        schools,
        offers,
        areas: vec![],
        times: vec![],
    };
    // Validate and handle capacity-sufficient results before analytical queries.
    let initial = calculate(input.clone(), params)?;
    if initial.summary.is_none() {
        return initial.render(params, BTreeMap::new());
    }
    let areas=diesel::sql_query(r#"SELECT z.kod AS code,z.nazev AS name,z.kod_obce AS municipality,
                o.nazev_obce AS municipality_name,o.kod_orp AS orp,o.nazev_orp AS orp_name,
                d.population,d.children,d.age_groups FROM "ZSJ" z LEFT JOIN "SIMULATION_OBCE" o USING(kod_obce)
                LEFT JOIN (SELECT kod_zsj,sum(populace)::bigint AS population,
                    max(populace) FILTER(WHERE demo_skupina='1300100014') AS children,count(*)::bigint AS age_groups
                    FROM "DATA_DEMOGRAFIE_ZSJ" WHERE rok=2021 GROUP BY kod_zsj) d ON d.kod_zsj=z.kod ORDER BY z.kod"#)
                .load::<AreaData>(connection).map_err(database_error)?;
    input.areas = areas
        .into_iter()
        .map(|a| {
            if a.age_groups != Some(21) {
                return Err(internal_error());
            }
            Ok(Area {
                code: a.code,
                name: a.name,
                municipality: a.municipality,
                municipality_name: a.municipality_name,
                orp: a.orp,
                orp_name: a.orp_name,
                children: f64::from(a.children.ok_or_else(internal_error)?) / 5.0,
                population: a.population.ok_or_else(internal_error)? as f64,
            })
        })
        .collect::<Result<Vec<_>, StubError>>()?;
    let slot = match params.scenar {
        requests::Scenar::Rano => "07:00-08:00",
    };
    input.times = times::table
        .filter(times::slot_prijezdu.eq(slot))
        .select(models::DojezdovaDoba::as_select())
        .load::<models::DojezdovaDoba>(connection)
        .map_err(database_error)?
        .into_iter()
        .map(|t| (t.kod_zsj, t.redizo, t.doba_jizdy.map(f64::from)))
        .collect();
    let result = calculate(input, params)?;
    let mut geometries = BTreeMap::new();
    if params.format == requests::Format::Geojson {
        let sql = match params.uroven {
            requests::Uroven::Zsj => {
                r#"SELECT kod AS code,ST_AsGeoJSON(boundary,15,0) AS geometry FROM "ZSJ""#
            }
            requests::Uroven::Obec => {
                r#"SELECT kod_obce AS code,ST_AsGeoJSON(ST_UnaryUnion(ST_Collect(ST_MakeValid(boundary))),15,0) AS geometry FROM "ZSJ" GROUP BY kod_obce"#
            }
            requests::Uroven::Orp => {
                r#"SELECT o.kod_orp AS code,ST_AsGeoJSON(ST_UnaryUnion(ST_Collect(ST_MakeValid(z.boundary))),15,0) AS geometry FROM "ZSJ" z JOIN "SIMULATION_OBCE" o USING(kod_obce) GROUP BY o.kod_orp"#
            }
        };
        for row in diesel::sql_query(sql)
            .load::<GeometryData>(connection)
            .map_err(database_error)?
        {
            geometries.insert(
                row.code,
                serde_json::from_str(&row.geometry).map_err(|_| internal_error())?,
            );
        }
    }
    result.render(params, geometries)
}
