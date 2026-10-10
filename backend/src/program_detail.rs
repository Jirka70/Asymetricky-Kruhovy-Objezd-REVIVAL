//! Program balance, per-school coverage, candidate gains, and profession demand.
use crate::{
    catalog::{database_error, internal_error, offering, read, vhodnost},
    contract::{StubError, api_error},
    db::DbPool,
    dto, models, programs, reachability, requests, schema,
};
use axum::http::StatusCode;
use diesel::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

// Coverage threshold is documented in the detail endpoint contract.
const LOW_COVERAGE_PERCENT: f64 = 50.0;

struct Snapshot {
    program: Option<models::Obor>,
    schools: Vec<models::Skola>,
    offers: Vec<models::NabidkaOboru>,
    areas: Vec<String>,
    population: Vec<(String, i32)>,
    times: Vec<models::DojezdovaDoba>,
    professions: Vec<(models::OborProfese, models::ProfesniSkupina)>,
    jobs: Vec<(String, String, i32)>,
}
impl Snapshot {
    fn load(
        connection: &mut PgConnection,
        code: &str,
        scenario: requests::Scenar,
    ) -> QueryResult<Self> {
        use schema::{
            data_demografie_zsj as demo, dojezdove_doby as times, nabidka_oboru,
            obor_profese as mapping, obory, poptavka_profesi as demand, profesni_skupiny as groups,
            stredni_skoly, zamestnavatele as workplaces, zsj,
        };
        let program = obory::table
            .find(code)
            .select(models::Obor::as_select())
            .first(connection)
            .optional()?;
        // Unknown programs should return 404 even when analytical data is incomplete.
        if program.is_none() {
            return Ok(Self {
                program,
                schools: vec![],
                offers: vec![],
                areas: vec![],
                population: vec![],
                times: vec![],
                professions: vec![],
                jobs: vec![],
            });
        }
        let schools = stredni_skoly::table
            .order(stredni_skoly::redizo)
            .select(models::Skola::as_select())
            .load(connection)?;
        // Other programs are needed for the candidate's related KKOV-group flag.
        let offers = nabidka_oboru::table
            .order((
                nabidka_oboru::redizo,
                nabidka_oboru::forma_studia,
                nabidka_oboru::id,
            ))
            .select(models::NabidkaOboru::as_select())
            .load(connection)?;
        let areas = zsj::table
            .order(zsj::kod)
            .select(zsj::kod)
            .load(connection)?;
        let population = demo::table
            .filter(demo::rok.eq(2021))
            .filter(demo::demo_skupina.eq("1300100014"))
            .select((demo::kod_zsj, demo::populace))
            .load(connection)?;
        let slot = match scenario {
            requests::Scenar::Rano => "07:00-08:00",
        };
        let times = times::table
            .filter(times::slot_prijezdu.eq(slot))
            .select(models::DojezdovaDoba::as_select())
            .load(connection)?;
        let professions = mapping::table
            .inner_join(groups::table)
            .filter(mapping::kod_oboru.eq(code))
            .order(mapping::cz_isco3)
            .select((
                models::OborProfese::as_select(),
                models::ProfesniSkupina::as_select(),
            ))
            .load(connection)?;
        let jobs = demand::table
            .inner_join(workplaces::table)
            .inner_join(mapping::table.on(mapping::cz_isco3.eq(demand::cz_isco3)))
            .filter(mapping::kod_oboru.eq(code))
            .filter(mapping::vhodnost.le(2_i16))
            .filter(demand::pocet_mist.gt(0))
            .filter(demand::min_vzdelani.ne_all(["vyssOdbor", "bakal", "vysoka", "doktor"]))
            .select((demand::cz_isco3, workplaces::ico, demand::pocet_mist))
            .load(connection)?;
        Ok(Self {
            program,
            schools,
            offers,
            areas,
            population,
            times,
            professions,
            jobs,
        })
    }
}

pub(crate) async fn get(
    pool: DbPool,
    code: String,
    params: requests::OborQuery,
) -> Result<dto::DetailOboru, StubError> {
    read(pool, move |connection| {
        let data = connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run(|connection| Snapshot::load(connection, &code, params.scenar))
            .map_err(database_error)?;
        build(data, params)
    })
    .await
}

fn build(data: Snapshot, params: requests::OborQuery) -> Result<dto::DetailOboru, StubError> {
    let program = data.program.ok_or_else(|| {
        api_error(
            StatusCode::NOT_FOUND,
            "obor_nenalezen",
            "Obor nebyl nalezen.",
        )
    })?;
    let level: dto::Stupen = serde_json::from_value(serde_json::Value::String(
        program
            .kod
            .chars()
            .nth(6)
            .ok_or_else(internal_error)?
            .to_string(),
    ))
    .map_err(|_| internal_error())?;
    let schools: BTreeMap<_, _> = data
        .schools
        .into_iter()
        .map(|s| (s.redizo.clone(), s))
        .collect();
    let mut existing = BTreeSet::new();
    let mut related = BTreeSet::new();
    let mut offers = Vec::new();
    for offer in data.offers {
        if offer.kod_oboru == program.kod {
            if !schools.contains_key(&offer.redizo) {
                return Err(internal_error());
            }
            existing.insert(offer.redizo.clone());
            offers.push(offer);
        } else if offer.kod_oboru.get(..2) == program.kod.get(..2) {
            related.insert(offer.redizo);
        }
    }
    let population: BTreeMap<_, _> = data.population.into_iter().collect();
    let times: BTreeMap<_, _> = data
        .times
        .into_iter()
        .map(|t| ((t.kod_zsj, t.redizo), t.doba_jizdy))
        .collect();
    let mut school_children: BTreeMap<_, i64> =
        schools.keys().map(|code| (code.clone(), 0)).collect();
    let mut gains = school_children.clone();
    let mut total_children = 0;
    let mut covered_children = 0;
    for area in data.areas {
        let children = reachability::cohort(*population.get(&area).ok_or_else(internal_error)?)?;
        total_children += children;
        let mut reachable = BTreeSet::new();
        for school in schools.keys() {
            let duration = *times
                .get(&(area.clone(), school.clone()))
                .ok_or_else(internal_error)?;
            if duration.is_some_and(|v| !v.is_finite() || v < 0.0) {
                return Err(internal_error());
            }
            if duration.is_some_and(|v| v <= f32::from(params.max_min)) {
                reachable.insert(school);
                *school_children.get_mut(school).ok_or_else(internal_error)? += children;
            }
        }
        // Count union coverage once, even when multiple offerings/schools overlap.
        if reachable.iter().any(|school| existing.contains(*school)) {
            covered_children += children;
        } else {
            for school in reachable {
                *gains.get_mut(school).ok_or_else(internal_error)? += children;
            }
        }
    }
    let capacity: i64 = offers.iter().map(|o| i64::from(o.pocet_prijimanych)).sum();
    let applications: i64 = offers
        .iter()
        .map(|o| i64::from(o.loni_pocet_prihlasek))
        .sum();
    let mut profession_jobs = BTreeMap::<String, i64>::new();
    let mut companies = BTreeSet::new();
    let mut job_count = 0;
    for (profession, ico, count) in data.jobs {
        companies.insert(ico);
        job_count += i64::from(count);
        *profession_jobs.entry(profession).or_default() += i64::from(count);
    }
    let mapped = !data.professions.is_empty();
    let trh_prace = if mapped {
        Some(dto::DetailOboruTrhPrace {
            volna_mista: Some(job_count),
            zamestnavatelu: Some(companies.len() as i64),
            profese: Some(
                data.professions
                    .into_iter()
                    .map(|(mapping, group)| {
                        Ok(dto::DetailOboruTrhPraceProfeseItem {
                            pocet: Some(profession_jobs.remove(&group.cz_isco3).unwrap_or(0)),
                            cz_isco3: Some(group.cz_isco3),
                            vhodnost: Some(vhodnost(mapping.vhodnost)?),
                            nazev: Some(group.nazev),
                        })
                    })
                    .collect::<Result<Vec<_>, StubError>>()?,
            ),
        })
    } else {
        None
    };
    let applications_per_place = programs::ratio(applications, capacity);
    let pressure = applications_per_place.map(|v| v / programs::REGIONAL_APPLICATIONS_PER_PLACE);
    let jobs_per_place = mapped
        .then(|| programs::ratio(job_count, capacity))
        .flatten();
    let share = programs::ratio(covered_children * 100, total_children).unwrap_or(0.0);
    let mut signals = Vec::new();
    if pressure.is_some_and(|v| v >= programs::PRESSURE_THRESHOLD) {
        signals.push(dto::Signal::Pretlak);
    }
    if pressure.is_some_and(|v| v <= programs::LOW_INTEREST_THRESHOLD) {
        signals.push(dto::Signal::NizkyZajem);
    }
    if total_children > 0 && share < LOW_COVERAGE_PERCENT {
        signals.push(dto::Signal::SpatnaDostupnost);
    }
    if jobs_per_place.is_some_and(|v| v >= programs::JOB_DEMAND_THRESHOLD) {
        signals.push(dto::Signal::PoptavkaTrhu);
    }
    let mut candidates = Vec::new();
    for (redizo, school) in &schools {
        if existing.contains(redizo) {
            continue;
        }
        candidates.push(dto::DetailOboruKandidatiItem {
            redizo: redizo.clone(),
            nazev: school.nazev.clone().ok_or_else(internal_error)?,
            obec: None,
            nove_dosazene_deti: gains[redizo],
            ma_pribuzny_obor: related.contains(redizo),
        });
    }
    candidates.sort_by(|a, b| {
        b.nove_dosazene_deti
            .cmp(&a.nove_dosazene_deti)
            .then_with(|| a.nazev.to_lowercase().cmp(&b.nazev.to_lowercase()))
            .then_with(|| a.redizo.cmp(&b.redizo))
    });
    candidates.truncate(usize::from(params.kandidatu));
    let mut offerings = Vec::new();
    for offer in offers {
        let school = schools.get(&offer.redizo).ok_or_else(internal_error)?;
        let children = school_children[&offer.redizo];
        let redizo = offer.redizo.clone();
        let mapped = offering(
            offer,
            models::Obor {
                kod: program.kod.clone(),
                nazev: program.nazev.clone(),
            },
        )?;
        offerings.push(dto::DetailOboruNabidkyItem {
            kod_oboru: mapped.kod_oboru,
            nazev_oboru: mapped.nazev_oboru,
            zamereni: mapped.zamereni,
            stupen: Some(level.clone()),
            forma: match mapped.forma {
                dto::NabidkaForma::Den => dto::DetailOboruNabidkyItemForma::Den,
                dto::NabidkaForma::Dal => dto::DetailOboruNabidkyItemForma::Dal,
            },
            delka_let: mapped.delka_let,
            kapacita: mapped.kapacita,
            prihlasky: mapped.prihlasky,
            prijati: mapped.prijati,
            prihlasky_na_misto: mapped.prihlasky_na_misto,
            index_pretlaku: mapped
                .prihlasky_na_misto
                .map(|v| v / programs::REGIONAL_APPLICATIONS_PER_PLACE),
            redizo: Some(redizo),
            nazev_skoly: Some(school.nazev.clone().ok_or_else(internal_error)?),
            deti_v_dosahu_skoly: Some(children),
        });
    }
    Ok(dto::DetailOboru {
        obor: dto::BilanceOboru {
            kod: program.kod,
            nazev: program.nazev,
            stupen: level,
            pocet_skol: existing.len() as i64,
            kapacita: capacity,
            prihlasky: applications,
            prihlasky_na_misto: applications_per_place,
            index_pretlaku: pressure,
            deti_v_dosahu: covered_children,
            deti_bez_oboru: total_children - covered_children,
            podil_deti_v_dosahu: Some(share),
            mist_na_100_deti: programs::ratio(capacity * 100, covered_children),
            zamestnavatelu: mapped.then_some(companies.len() as i64),
            volna_mista: mapped.then_some(job_count),
            volna_mista_na_misto: jobs_per_place,
            signaly: signals,
        },
        nabidky: offerings,
        kandidati: candidates,
        trh_prace,
        meta: dto::MetaDosah {
            scenar: match params.scenar {
                requests::Scenar::Rano => "rano",
            }
            .to_owned(),
            max_min: i64::from(params.max_min),
        },
    })
}
