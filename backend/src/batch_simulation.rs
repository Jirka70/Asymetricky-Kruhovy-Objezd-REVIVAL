//! Simultaneous capacity changes for one program; never modifies database data.
use crate::{
    catalog::{internal_error, read},
    contract::{GeoJson, StubError, api_error},
    db::DbPool,
    dto, programs, requests,
    simulation::{self, Input, Row, in_limit, round},
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::collections::{BTreeMap, BTreeSet};

pub enum BatchResponse {
    Dictionary(dto::BatchSimulace),
    Geojson(dto::BatchSimulaceGeojson),
}
impl IntoResponse for BatchResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Dictionary(body) => Json(body).into_response(),
            Self::Geojson(body) => GeoJson(body).into_response(),
        }
    }
}
struct Prepared {
    before: BTreeMap<String, i64>,
    after: BTreeMap<String, i64>,
    changes: BTreeMap<String, i64>,
    interest: f64,
}
fn prepare(input: &Input, request: &requests::BatchSimulaceRequest) -> Result<Prepared, StubError> {
    if request.zmeny.is_empty() || request.zmeny.len() > 100 {
        return Err(StubError::invalid("zmeny"));
    }
    if request.max_min != 0 && !(10..=180).contains(&request.max_min) {
        return Err(StubError::invalid("max_min"));
    }
    if !input.program_exists {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "obor_nenalezen",
            "Obor nebyl nalezen.",
        ));
    }
    let mut changes = BTreeMap::<String, i64>::new();
    for change in &request.zmeny {
        if change.zmena_kapacity == 0 || !(-300..=300).contains(&change.zmena_kapacity) {
            return Err(StubError::invalid("zmeny"));
        }
        if !input.schools.contains_key(&change.redizo.0) {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "skola_nenalezena",
                "Škola nebyla nalezena.",
            ));
        }
        let value = changes.entry(change.redizo.0.clone()).or_default();
        *value = value
            .checked_add(change.zmena_kapacity)
            .ok_or_else(|| StubError::invalid("zmeny"))?;
    }
    let mut before = BTreeMap::<String, i64>::new();
    let mut applications = 0;
    let mut all_applications = 0;
    for offer in &input.offers {
        if offer.kapacita < 0 || offer.prihlasky < 0 {
            return Err(internal_error());
        }
        all_applications += offer.prihlasky;
        if offer.kod_oboru == request.obor.0 {
            if !input.schools.contains_key(&offer.redizo) {
                return Err(internal_error());
            }
            *before.entry(offer.redizo.clone()).or_default() += offer.kapacita;
            applications += offer.prihlasky;
        }
    }
    if before.is_empty() {
        return Err(api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "obor_bez_nabidky",
            "Obor nemá denní nabídky v kraji; nelze odhadnout zájem.",
        ));
    }
    let mut after = before.clone();
    for (school, delta) in &changes {
        let capacity = after.entry(school.clone()).or_default();
        *capacity = capacity
            .checked_add(*delta)
            .filter(|v| *v >= 0)
            .ok_or_else(|| StubError::invalid("zmeny"))?;
    }
    // A final zero means no available first-year offering in this scenario.
    after.retain(|_, capacity| *capacity > 0);
    Ok(Prepared {
        before,
        after,
        changes,
        interest: programs::ratio(applications, all_applications).unwrap_or(0.0),
    })
}
pub struct Calculation {
    rows: Vec<Row>,
    summary: dto::BatchSimulaceSouhrn,
    meta: dto::BatchSimulaceMeta,
}

pub fn calculate(
    input: Input,
    request: &requests::BatchSimulaceRequest,
) -> Result<Calculation, StubError> {
    let prepared = prepare(&input, request)?;
    let mut times = BTreeMap::<String, BTreeMap<String, Option<f64>>>::new();
    for (area, school, time) in input.times {
        if time.is_some_and(|t| !t.is_finite() || t < 0.0) {
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
    let participating: BTreeSet<_> = prepared
        .before
        .keys()
        .chain(prepared.after.keys())
        .chain(prepared.changes.keys())
        .cloned()
        .collect();
    let mut catchment_before = BTreeMap::<String, f64>::new();
    let mut catchment_after = BTreeMap::<String, f64>::new();
    let mut transfers = BTreeMap::<(Option<String>, Option<String>), (f64, f64)>::new();
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    let mut children_before = 0.0;
    let mut children_after = 0.0;
    let mut gained_children = 0.0;
    let mut lost_children = 0.0;
    let mut gained_demand = 0.0;
    let mut lost_demand = 0.0;
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
        for school in &participating {
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
        let before = nearest(&prepared.before);
        let after = nearest(&prepared.after);
        let before_time = before.as_ref().map(|v| v.0);
        let after_time = after.as_ref().map(|v| v.0);
        let before_reachable = in_limit(before_time, requests::travel_limit(request.max_min));
        let after_reachable = in_limit(after_time, requests::travel_limit(request.max_min));
        let before_school = before.as_ref().map(|v| v.1.clone());
        let after_school = after.as_ref().map(|v| v.1.clone());
        let demand = area.children * prepared.interest;
        if before_reachable {
            *catchment_before
                .entry(before_school.clone().ok_or_else(internal_error)?)
                .or_default() += demand;
            children_before += area.children;
        }
        if after_reachable {
            *catchment_after
                .entry(after_school.clone().ok_or_else(internal_error)?)
                .or_default() += demand;
            children_after += area.children;
        }
        let source = before_school.clone().filter(|_| before_reachable);
        let destination = after_school.clone().filter(|_| after_reachable);
        if source != destination {
            let transfer = transfers.entry((source, destination)).or_default();
            transfer.0 += area.children;
            transfer.1 += demand;
        }
        let gained = !before_reachable && after_reachable;
        let lost = before_reachable && !after_reachable;
        if gained {
            gained_children += area.children;
            gained_demand += demand;
        }
        if lost {
            lost_children += area.children;
            lost_demand += demand;
        }
        let improved = after_time.is_some_and(|a| before_time.is_none_or(|b| a < b));
        let worsened = before_time.is_some_and(|b| after_time.is_none_or(|a| a > b));
        rows.push(Row {
            changed: before_time != after_time || before_school != after_school,
            area,
            before: before_time,
            after: after_time,
            target: None,
            school_after: after_school,
            improved,
            worsened,
            new_reach: gained,
            lost_reach: lost,
            demand,
        });
    }
    let effects = participating
        .iter()
        .map(|school| {
            let capacity_before = prepared.before.get(school).copied().unwrap_or(0);
            let capacity_after = prepared.after.get(school).copied().unwrap_or(0);
            let before = catchment_before.get(school).copied().unwrap_or(0.0);
            let after = catchment_after.get(school).copied().unwrap_or(0.0);
            let mut incoming = 0.0;
            let mut outgoing = 0.0;
            let mut new = 0.0;
            let mut lost = 0.0;
            for ((source, destination), (_, demand)) in &transfers {
                if destination.as_ref() == Some(school) {
                    incoming += demand;
                    if source.is_none() {
                        new += demand;
                    }
                }
                if source.as_ref() == Some(school) {
                    outgoing += demand;
                    if destination.is_none() {
                        lost += demand;
                    }
                }
            }
            dto::BatchBilanceSkoly {
                redizo: school.clone(),
                nazev: input.schools[school].clone(),
                zmena_kapacity: prepared.changes.get(school).copied().unwrap_or(0),
                kapacita_pred: capacity_before,
                kapacita_po: capacity_after,
                spad_pred: round(before, 1),
                spad_po: round(after, 1),
                bilance_pred: round(capacity_before as f64 - before, 1),
                bilance_po: round(capacity_after as f64 - after, 1),
                prichozi_uchazeci: round(incoming, 1),
                odchozi_uchazeci: round(outgoing, 1),
                novi_v_dosahu: round(new, 1),
                ztraceni_v_dosahu: round(lost, 1),
            }
        })
        .collect();
    let summary = dto::BatchSimulaceSouhrn {
        jednotek_celkem: 0,
        zlepsenych_jednotek: 0,
        zhorsenych_jednotek: 0,
        v_limitu: vec![],
        prumerne_zkraceni_min: 0.0,
        prumerne_prodlouzeni_min: 0.0,
        deti_v_dosahu_pred: round(children_before, 1),
        deti_v_dosahu_po: round(children_after, 1),
        nove_dosazene_deti: round(gained_children, 1),
        ztracene_deti: round(lost_children, 1),
        novi_v_dosahu: round(gained_demand, 1),
        ztraceni_v_dosahu: round(lost_demand, 1),
        kapacita_pred: prepared.before.values().sum(),
        kapacita_po: prepared.after.values().sum(),
        bilance_skol: effects,
        presuny: transfers
            .into_iter()
            .map(|((odkud, kam), (deti, uchazeci))| dto::BatchPresun {
                odkud,
                kam,
                deti: round(deti, 1),
                uchazeci: round(uchazeci, 1),
            })
            .collect(),
    };
    let meta = dto::BatchSimulaceMeta {
        obor: request.obor.0.clone(),
        zmeny: prepared
            .changes
            .into_iter()
            .map(|(redizo, zmena_kapacity)| requests::ZmenaKapacity {
                redizo: requests::Redizo(redizo),
                zmena_kapacity,
            })
            .collect(),
        max_min: i64::from(request.max_min),
        scenar: request.scenar,
        uroven: match request.uroven {
            requests::Uroven::Zsj => dto::SimulaceMetaUroven::Zsj,
            requests::Uroven::Obec => dto::SimulaceMetaUroven::Obec,
            requests::Uroven::Orp => dto::SimulaceMetaUroven::Orp,
        },
        podil_zajmu: round(prepared.interest, 4),
    };
    Ok(Calculation {
        rows,
        summary,
        meta,
    })
}
fn area_value(area: &simulation::OutputArea) -> dto::BatchSimulacePlocha {
    let p = &area.value;
    dto::BatchSimulacePlocha {
        nazev: p.nazev.clone(),
        cas_min_puvodni: p.cas_min_puvodni,
        cas_min: p.cas_min,
        zlepseni_min: p.zlepseni_min,
        pasmo_puvodni: p
            .pasmo_puvodni
            .clone()
            .expect("aggregate sets original band"),
        pasmo: p.pasmo.clone(),
        deti: p.deti,
        potencialni_uchazeci: p.potencialni_uchazeci,
        novy_dosah: p.novy_dosah,
        ztraceny_dosah: area.lost_reach,
        zlepseno: area.improved,
        zhorseno: area.worsened,
    }
}
impl Calculation {
    pub fn render(
        mut self,
        request: &requests::BatchSimulaceRequest,
        geometries: BTreeMap<String, dto::GeoPlocha>,
    ) -> Result<BatchResponse, StubError> {
        let areas = simulation::aggregate(&self.rows, request.uroven, request.max_min)?;
        self.summary.jednotek_celkem = areas.len() as i64;
        self.summary.zlepsenych_jednotek = areas.iter().filter(|a| a.improved).count() as i64;
        self.summary.zhorsenych_jednotek = areas.iter().filter(|a| a.worsened).count() as i64;
        let differences: Vec<f64> = areas
            .iter()
            .filter_map(|a| a.before.zip(a.after).map(|(b, a)| b - a))
            .collect();
        let positive: Vec<_> = differences.iter().copied().filter(|v| *v > 0.0).collect();
        let negative: Vec<_> = differences
            .iter()
            .copied()
            .filter(|v| *v < 0.0)
            .map(|v| -v)
            .collect();
        self.summary.prumerne_zkraceni_min = if positive.is_empty() {
            0.0
        } else {
            round(positive.iter().sum::<f64>() / positive.len() as f64, 1)
        };
        self.summary.prumerne_prodlouzeni_min = if negative.is_empty() {
            0.0
        } else {
            round(negative.iter().sum::<f64>() / negative.len() as f64, 1)
        };
        let limits: BTreeSet<i64> = [30, 45, 60, i64::from(request.max_min)]
            .into_iter()
            .collect();
        self.summary.v_limitu = limits
            .into_iter()
            .map(|limit| dto::VLimituPredPo {
                limit_min: limit,
                pred: areas
                    .iter()
                    .filter(|a| in_limit(a.before, requests::travel_limit(limit as u16)))
                    .count() as i64,
                po: areas
                    .iter()
                    .filter(|a| in_limit(a.after, requests::travel_limit(limit as u16)))
                    .count() as i64,
            })
            .collect();
        match request.format {
            requests::Format::Slovnik => Ok(BatchResponse::Dictionary(dto::BatchSimulace {
                jednotky: areas
                    .iter()
                    .filter(|a| a.changed)
                    .map(|a| (a.code.clone(), area_value(a)))
                    .collect(),
                souhrn: self.summary,
                meta: self.meta,
            })),
            requests::Format::Geojson => {
                let features = areas
                    .iter()
                    .map(|a| {
                        Ok(dto::BatchSimulaceFeature {
                            r#type: dto::SimulaceGeojsonFeaturesItemType::Feature,
                            geometry: geometries
                                .get(&a.code)
                                .cloned()
                                .ok_or_else(internal_error)?,
                            properties: dto::BatchSimulaceProperties {
                                kod: a.code.clone(),
                                uroven: self.meta.uroven.clone(),
                                v_dosahu: in_limit(
                                    a.after,
                                    requests::travel_limit(request.max_min),
                                ),
                                deti_v_dosahu: round(a.children_reached, 1),
                                podil_deti_v_dosahu: if a.value.deti > 0.0 {
                                    round(100.0 * a.children_reached / a.value.deti, 2)
                                } else {
                                    0.0
                                },
                                nejblizsi_redizo: a.school.clone(),
                                simulace: area_value(a),
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, StubError>>()?;
                Ok(BatchResponse::Geojson(dto::BatchSimulaceGeojson {
                    r#type: dto::SimulaceGeojsonType::FeatureCollection,
                    features,
                    souhrn: self.summary,
                    meta: self.meta,
                }))
            }
        }
    }
}
pub(crate) async fn run(
    pool: DbPool,
    request: requests::BatchSimulaceRequest,
) -> Result<BatchResponse, StubError> {
    read(pool, move |connection| {
        connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run::<_, StubError, _>(|connection| {
                let mut input = simulation::load_base(connection, &request.obor.0, None)?;
                prepare(&input, &request)?;
                simulation::load_analytical(connection, &mut input, request.scenar)?;
                let calculation = calculate(input, &request)?;
                let geometries = if request.format == requests::Format::Geojson {
                    simulation::load_geometries(connection, request.uroven)?
                } else {
                    BTreeMap::new()
                };
                calculation.render(&request, geometries)
            })
    })
    .await
}
