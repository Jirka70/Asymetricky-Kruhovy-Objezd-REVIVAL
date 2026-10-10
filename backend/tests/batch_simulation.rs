use obor_backend::{
    batch_simulation::{self, BatchResponse},
    requests::BatchSimulaceRequest,
    simulation::{Area, Input, Offer},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const A: &str = "600000001";
const B: &str = "600000002";
const OBOR: &str = "23-68-H/01";
fn input() -> Input {
    Input {
        program_exists: true,
        schools: BTreeMap::from([(A.into(), "A".into()), (B.into(), "B".into())]),
        offers: vec![
            Offer {
                redizo: A.into(),
                kod_oboru: OBOR.into(),
                kapacita: 30,
                prihlasky: 60,
            },
            Offer {
                redizo: A.into(),
                kod_oboru: OBOR.into(),
                kapacita: 20,
                prihlasky: 40,
            },
            Offer {
                redizo: B.into(),
                kod_oboru: "18-20-M/01".into(),
                kapacita: 50,
                prihlasky: 100,
            },
        ],
        areas: (0..4)
            .map(|i| Area {
                code: format!("00000{i}"),
                name: format!("Area {i}"),
                municipality: Some("554961".into()),
                municipality_name: Some("Town".into()),
                orp: Some("4101".into()),
                orp_name: Some("Region".into()),
                children: (i + 1) as f64 * 10.0,
                population: (i + 1) as f64 * 100.0,
            })
            .collect(),
        times: [
            (Some(10.), Some(20.)),
            (Some(100.), Some(10.)),
            (Some(20.), None),
            (None, Some(30.)),
        ]
        .into_iter()
        .enumerate()
        .flat_map(|(i, (a, b))| {
            [
                (format!("00000{i}"), A.into(), a),
                (format!("00000{i}"), B.into(), b),
            ]
        })
        .collect(),
    }
}
fn request(changes: Value) -> BatchSimulaceRequest {
    serde_json::from_value(json!({"obor":OBOR,"max_min":60,"uroven":"zsj","zmeny":changes}))
        .unwrap()
}
fn value(input: Input, request: &BatchSimulaceRequest) -> Value {
    match batch_simulation::calculate(input, request)
        .unwrap()
        .render(request, BTreeMap::new())
        .unwrap()
    {
        BatchResponse::Dictionary(v) => serde_json::to_value(v).unwrap(),
        _ => panic!("dictionary"),
    }
}
fn move_request() -> BatchSimulaceRequest {
    request(json!([{"redizo":A,"zmena_kapacity":-50},{"redizo":B,"zmena_kapacity":50}]))
}
#[test]
fn transfer_reports_gains_losses_worsening_and_school_balances() {
    let v = value(input(), &move_request());
    let s = &v["souhrn"];
    assert_eq!(s["kapacita_pred"], 50);
    assert_eq!(s["kapacita_po"], 50);
    assert_eq!(s["deti_v_dosahu_pred"], 40.0);
    assert_eq!(s["deti_v_dosahu_po"], 70.0);
    assert_eq!(s["nove_dosazene_deti"], 60.0);
    assert_eq!(s["ztracene_deti"], 30.0);
    assert_eq!(s["novi_v_dosahu"], 30.0);
    assert_eq!(s["ztraceni_v_dosahu"], 15.0);
    assert_eq!(s["zlepsenych_jednotek"], 2);
    assert_eq!(s["zhorsenych_jednotek"], 2);
    assert_eq!(s["prumerne_zkraceni_min"], 90.0);
    assert_eq!(s["prumerne_prodlouzeni_min"], 10.0);
    assert_eq!(v["jednotky"]["000000"]["zlepseni_min"], -10.0);
    assert_eq!(v["jednotky"]["000002"]["ztraceny_dosah"], true);
    assert_eq!(v["jednotky"]["000002"]["cas_min"], Value::Null);
    assert_eq!(v["jednotky"]["000003"]["novy_dosah"], true);
    assert_eq!(v["jednotky"]["000003"]["zlepseni_min"], Value::Null);
    let schools = s["bilance_skol"].as_array().unwrap();
    assert_eq!(schools[0]["spad_pred"], 20.0);
    assert_eq!(schools[0]["spad_po"], 0.0);
    assert_eq!(schools[0]["odchozi_uchazeci"], 20.0);
    assert_eq!(schools[1]["spad_po"], 35.0);
    assert_eq!(schools[1]["bilance_po"], 15.0);
    assert_eq!(schools[1]["prichozi_uchazeci"], 35.0);
    assert_eq!(
        s["presuny"],
        json!([{"odkud":null,"kam":B,"deti":60.0,"uchazeci":30.0},{"odkud":A,"kam":null,"deti":30.0,"uchazeci":15.0},{"odkud":A,"kam":B,"deti":10.0,"uchazeci":5.0}])
    );
}
#[test]
fn changes_are_simultaneous_and_duplicate_entries_are_merged() {
    let r = move_request();
    let expected = value(input(), &r);
    let mut reverse = r.clone();
    reverse.zmeny.reverse();
    assert_eq!(value(input(), &reverse), expected);
    let merged = request(
        json!([{"redizo":A,"zmena_kapacity":-100},{"redizo":B,"zmena_kapacity":50},{"redizo":A,"zmena_kapacity":50}]),
    );
    assert_eq!(value(input(), &merged), expected);
    let cancelled =
        request(json!([{"redizo":A,"zmena_kapacity":-30},{"redizo":A,"zmena_kapacity":30}]));
    let v = value(input(), &cancelled);
    assert_eq!(v["jednotky"], json!({}));
    assert_eq!(v["souhrn"]["presuny"], json!([]));
    assert_eq!(v["meta"]["zmeny"][0]["zmena_kapacity"], 0);
}
#[test]
fn partial_reduction_changes_capacity_and_balance_only() {
    let v = value(
        input(),
        &request(json!([{"redizo":A,"zmena_kapacity":-10}])),
    );
    assert_eq!(v["jednotky"], json!({}));
    assert_eq!(v["souhrn"]["kapacita_po"], 40);
    assert_eq!(v["souhrn"]["bilance_skol"][0]["spad_po"], 20.0);
    assert_eq!(v["souhrn"]["bilance_skol"][0]["bilance_po"], 20.0);
    assert_eq!(v["souhrn"]["presuny"], json!([]));
}
#[test]
fn last_offering_can_be_removed_and_zero_applications_are_valid() {
    let r = request(json!([{"redizo":A,"zmena_kapacity":-50}]));
    let v = value(input(), &r);
    assert_eq!(v["souhrn"]["kapacita_po"], 0);
    assert_eq!(v["souhrn"]["deti_v_dosahu_po"], 0.0);
    assert_eq!(v["souhrn"]["ztracene_deti"], 40.0);
    let mut zero = input();
    for offer in &mut zero.offers {
        offer.prihlasky = 0;
    }
    let v = value(zero, &move_request());
    assert_eq!(v["meta"]["podil_zajmu"], 0.0);
    assert_eq!(v["souhrn"]["novi_v_dosahu"], 0.0);
}
#[test]
fn aggregated_units_keep_mixed_changes_and_population_weights() {
    let mut r = move_request();
    r.uroven = obor_backend::requests::Uroven::Obec;
    let v = value(input(), &r);
    let p = &v["jednotky"]["554961"];
    assert_eq!(p["cas_min_puvodni"], 45.0);
    assert_eq!(p["cas_min"], 22.86);
    assert_eq!(p["deti"], 100.0);
    assert_eq!(p["zlepseno"], true);
    assert_eq!(p["zhorseno"], true);
    assert_eq!(p["novy_dosah"], true);
    assert_eq!(p["ztraceny_dosah"], true);
    assert_eq!(v["souhrn"]["jednotek_celkem"], 1);
    assert_eq!(v["souhrn"]["deti_v_dosahu_po"], 70.0);
}
#[test]
fn invalid_inputs_are_rejected_but_explicit_null_times_are_valid() {
    let r = move_request();
    for changes in [
        json!([]),
        json!([{"redizo":A,"zmena_kapacity":0}]),
        json!([{"redizo":A,"zmena_kapacity":301}]),
        json!([{"redizo":A,"zmena_kapacity":-51}]),
        json!([{"redizo":"600000099","zmena_kapacity":1}]),
    ] {
        assert!(batch_simulation::calculate(input(), &request(changes)).is_err());
    }
    let mut missing = input();
    missing.times.remove(0);
    assert!(batch_simulation::calculate(missing, &r).is_err());
    let mut duplicate = input();
    duplicate.times.push(duplicate.times[0].clone());
    assert!(batch_simulation::calculate(duplicate, &r).is_err());
    let mut invalid = input();
    invalid.times[0].2 = Some(f64::NAN);
    assert!(batch_simulation::calculate(invalid, &r).is_err());
    let mut absent = input();
    absent.program_exists = false;
    assert!(batch_simulation::calculate(absent, &r).is_err());
    let mut empty = input();
    empty.offers.clear();
    assert!(batch_simulation::calculate(empty, &r).is_err());
    let mut negative = input();
    negative.areas[0].children = -1.0;
    assert!(batch_simulation::calculate(negative, &r).is_err());
    assert!(batch_simulation::calculate(input(), &r).is_ok());
}
#[test]
fn full_geojson_includes_unchanged_areas_and_requires_geometry() {
    let mut r = request(json!([{"redizo":A,"zmena_kapacity":-10}]));
    r.format = obor_backend::requests::Format::Geojson;
    assert!(
        batch_simulation::calculate(input(), &r)
            .unwrap()
            .render(&r, BTreeMap::new())
            .is_err()
    );
    let geometry: obor_backend::dto::GeoPlocha = serde_json::from_value(
        json!({"type":"Polygon","coordinates":[[[12.0,50.0],[13.0,50.0],[13.0,51.0],[12.0,50.0]]]}),
    )
    .unwrap();
    let geometries = input()
        .areas
        .into_iter()
        .map(|a| (a.code, geometry.clone()))
        .collect();
    match batch_simulation::calculate(input(), &r)
        .unwrap()
        .render(&r, geometries)
        .unwrap()
    {
        BatchResponse::Geojson(v) => {
            assert_eq!(v.features.len(), 4);
            assert_eq!(
                v.features[0].properties.nejblizsi_redizo.as_deref(),
                Some(A)
            );
        }
        _ => panic!("GeoJSON"),
    }
}

#[test]
fn nearest_school_ties_change_assignment_without_fabricating_time_improvements() {
    let mut data = input();
    // B teaches first, A is added with the same travel time but wins the REDIZO tie.
    for offer in &mut data.offers {
        if offer.kod_oboru == OBOR {
            offer.redizo = B.into();
        }
    }
    data.areas.truncate(1);
    for (_, school, time) in &mut data.times {
        if school == B {
            *time = Some(10.0);
        }
    }
    let r = request(json!([{"redizo":A,"zmena_kapacity":1}]));
    let v = value(data, &r);
    assert_eq!(v["jednotky"]["000000"]["zlepseni_min"], 0.0);
    assert_eq!(v["jednotky"]["000000"]["zlepseno"], false);
    assert_eq!(v["souhrn"]["zlepsenych_jednotek"], 0);
    assert_eq!(
        v["souhrn"]["presuny"],
        json!([{"odkud":B,"kam":A,"deti":10.0,"uchazeci":5.0}])
    );
}

#[test]
fn fractional_cutoffs_and_cohorts_are_not_rounded_before_computing_access() {
    let mut data = input();
    data.areas.truncate(1);
    data.areas[0].children = 2.6;
    for (_, school, time) in &mut data.times {
        if school == A {
            *time = Some(60.001);
        } else {
            *time = Some(60.0);
        }
    }
    let r = request(json!([{"redizo":B,"zmena_kapacity":1}]));
    let v = value(data, &r);
    assert_eq!(v["souhrn"]["deti_v_dosahu_pred"], 0.0);
    assert_eq!(v["souhrn"]["deti_v_dosahu_po"], 2.6);
    assert_eq!(v["souhrn"]["novi_v_dosahu"], 1.3);
    assert_eq!(v["jednotky"]["000000"]["novy_dosah"], true);
}

#[test]
fn unlimited_includes_long_journeys_and_preserves_unknown_connections() {
    let mut input = input();
    for (_, _, time) in &mut input.times {
        if let Some(t) = time {
            *t += 240.0;
        }
    }
    let mut request = move_request();
    request.max_min = 180;
    assert_eq!(
        value(input.clone(), &request)["souhrn"]["deti_v_dosahu_pred"],
        0.0
    );
    request.max_min = 0;
    let result = value(input, &request);
    assert_eq!(result["meta"]["max_min"], 0);
    assert_eq!(result["souhrn"]["deti_v_dosahu_pred"], 60.0);
    assert_eq!(result["souhrn"]["deti_v_dosahu_po"], 70.0);
    assert_eq!(
        result["souhrn"]["v_limitu"][0],
        json!({"limit_min":0,"pred":3,"po":3})
    );
}
