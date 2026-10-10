//! Program catalog and admissions/job-demand totals; no travel-time calculations.
use crate::{
    catalog::{database_error, internal_error, read},
    contract::StubError,
    db::DbPool,
    dto, models, requests, schema,
};
use diesel::prelude::*;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

// Admissions baseline and signal thresholds from the public API contract.
pub(crate) const REGIONAL_APPLICATIONS_PER_PLACE: f64 = 2.66;
pub(crate) const PRESSURE_THRESHOLD: f64 = 1.5;
pub(crate) const LOW_INTEREST_THRESHOLD: f64 = 0.5;
pub(crate) const JOB_DEMAND_THRESHOLD: f64 = 2.0;

pub(crate) async fn list(
    pool: DbPool,
    params: requests::OboryQuery,
) -> Result<dto::Obory, StubError> {
    let signals = params
        .signal
        .as_ref()
        .map(|filter| {
            filter
                .0
                .iter()
                .map(|value| match value.as_str() {
                    "pretlak" => Ok(dto::OborSignal::Pretlak),
                    "nizky_zajem" => Ok(dto::OborSignal::NizkyZajem),
                    "poptavka_trhu" => Ok(dto::OborSignal::PoptavkaTrhu),
                    _ => Err(StubError::invalid("signal")),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    read(pool, move |connection| {
        // All inputs come from one consistent database snapshot, with a constant
        // number of queries rather than one query per program.
        let snapshot = connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run(|connection| Snapshot::load(connection, params.forma))
            .map_err(database_error)?;
        let mut totals = BTreeMap::<String, OfferingTotals>::new();
        for offer in snapshot.offers {
            let total = totals.entry(offer.kod_oboru).or_default();
            total.schools.insert(offer.redizo);
            total.capacity += i64::from(offer.pocet_prijimanych);
            total.applications += i64::from(offer.loni_pocet_prihlasek);
        }
        let mut jobs = BTreeMap::<String, JobTotals>::new();
        for (code, ico, count) in snapshot.jobs {
            let total = jobs.entry(code).or_default();
            total.companies.insert(ico);
            total.count += i64::from(count);
        }
        let mut data = Vec::new();
        for program in snapshot.programs {
            // forma selects programs with offerings in that form. A zero-capacity
            // offering still belongs to the catalog; it produces null ratios.
            let Some(total) = totals.remove(&program.kod) else {
                continue;
            };
            let level: dto::Stupen = serde_json::from_value(serde_json::Value::String(
                program
                    .kod
                    .chars()
                    .nth(6)
                    .ok_or_else(internal_error)?
                    .to_string(),
            ))
            .map_err(|_| internal_error())?;
            if params
                .stupen
                .as_ref()
                .is_some_and(|levels| !levels.0.contains(&level))
            {
                continue;
            }
            let demand = jobs.remove(&program.kod).unwrap_or_default();
            let mapped = snapshot.mapped.contains(&program.kod);
            let applications_per_place = ratio(total.applications, total.capacity);
            let pressure =
                applications_per_place.map(|value| value / REGIONAL_APPLICATIONS_PER_PLACE);
            let jobs_per_place = mapped
                .then(|| ratio(demand.count, total.capacity))
                .flatten();
            let mut row_signals = Vec::new();
            if pressure.is_some_and(|value| value >= PRESSURE_THRESHOLD) {
                row_signals.push(dto::OborSignal::Pretlak);
            }
            if pressure.is_some_and(|value| value <= LOW_INTEREST_THRESHOLD) {
                row_signals.push(dto::OborSignal::NizkyZajem);
            }
            if jobs_per_place.is_some_and(|value| value >= JOB_DEMAND_THRESHOLD) {
                row_signals.push(dto::OborSignal::PoptavkaTrhu);
            }
            // CSV signals use OR: return a program with any requested signal.
            if signals
                .as_ref()
                .is_some_and(|wanted| !wanted.iter().any(|signal| row_signals.contains(signal)))
            {
                continue;
            }
            data.push(dto::OborPrehled {
                kod: program.kod,
                nazev: program.nazev,
                stupen: level,
                pocet_skol: total.schools.len() as i64,
                kapacita: total.capacity,
                prihlasky: total.applications,
                prihlasky_na_misto: applications_per_place,
                index_pretlaku: pressure,
                zamestnavatelu: mapped.then_some(demand.companies.len() as i64),
                volna_mista: mapped.then_some(demand.count),
                volna_mista_na_misto: jobs_per_place,
                signaly: row_signals,
            });
        }
        data.sort_by(|left, right| {
            let numeric = match params.razeni {
                requests::Razeni::Nazev => Ordering::Equal,
                requests::Razeni::IndexPretlaku => {
                    compare_nullable(left.index_pretlaku, right.index_pretlaku, false)
                }
                requests::Razeni::IndexPretlakuSestupne => {
                    compare_nullable(left.index_pretlaku, right.index_pretlaku, true)
                }
                requests::Razeni::VolnaMistaNaMisto => {
                    compare_nullable(left.volna_mista_na_misto, right.volna_mista_na_misto, false)
                }
                requests::Razeni::VolnaMistaNaMistoSestupne => {
                    compare_nullable(left.volna_mista_na_misto, right.volna_mista_na_misto, true)
                }
            };
            numeric
                .then_with(|| left.nazev.to_lowercase().cmp(&right.nazev.to_lowercase()))
                .then_with(|| left.kod.cmp(&right.kod))
        });
        Ok(dto::Obory {
            data,
            meta: dto::OboryMeta {
                prumer_prihlasek_na_misto: REGIONAL_APPLICATIONS_PER_PLACE,
                prijimaci_rizeni: dto::MetaBilancePrijimaciRizeni {
                    rok: Some(2026),
                    kolo: Some(1),
                },
                prahy: dto::OboryPrahy {
                    pretlak: PRESSURE_THRESHOLD,
                    nizky_zajem: LOW_INTEREST_THRESHOLD,
                    poptavka_trhu: JOB_DEMAND_THRESHOLD,
                },
            },
        })
    })
    .await
}

struct Snapshot {
    programs: Vec<models::Obor>,
    offers: Vec<models::NabidkaOboru>,
    mapped: BTreeSet<String>,
    jobs: Vec<(String, String, i32)>,
}
impl Snapshot {
    fn load(connection: &mut PgConnection, form: requests::Forma) -> QueryResult<Self> {
        use schema::{
            nabidka_oboru as offers, obor_profese as mapping, obory, poptavka_profesi as demand,
            zamestnavatele as workplaces,
        };
        let forma = match form {
            requests::Forma::Den => "den",
            requests::Forma::Dal => "dal",
        };
        let programs = obory::table
            .select(models::Obor::as_select())
            .load(connection)?;
        let offers = offers::table
            .filter(offers::forma_studia.eq(forma))
            .select(models::NabidkaOboru::as_select())
            .load(connection)?;
        let mapped = mapping::table
            .select(mapping::kod_oboru)
            .distinct()
            .load::<String>(connection)?
            .into_iter()
            .collect();
        // Each mapping is unique by (program, profession), and each demand row
        // by (workplace, profession, education). Sum each applicable row once.
        // Match the employer endpoint defaults: suitability 1/2 and no tertiary-only jobs.
        let jobs = demand::table
            .inner_join(workplaces::table)
            .inner_join(mapping::table.on(mapping::cz_isco3.eq(demand::cz_isco3)))
            .filter(mapping::vhodnost.le(2_i16))
            .filter(demand::pocet_mist.gt(0))
            .filter(demand::min_vzdelani.ne_all(["vyssOdbor", "bakal", "vysoka", "doktor"]))
            .select((mapping::kod_oboru, workplaces::ico, demand::pocet_mist))
            .load(connection)?;
        Ok(Self {
            programs,
            offers,
            mapped,
            jobs,
        })
    }
}

#[derive(Default)]
struct OfferingTotals {
    schools: BTreeSet<String>,
    capacity: i64,
    applications: i64,
}
#[derive(Default)]
struct JobTotals {
    companies: BTreeSet<String>,
    count: i64,
}
pub(crate) fn ratio(numerator: i64, denominator: i64) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}
// Missing data stays last for both ascending and descending order.
fn compare_nullable(left: Option<f64>, right: Option<f64>, descending: bool) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) if descending => right.total_cmp(&left),
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
