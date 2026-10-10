use obor_backend::{
    contract::SimulaceResponse,
    dto, requests,
    simulation::{self, Input},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn value(input: Input, params: &requests::SimulaceQuery) -> Value {
    match simulation::calculate(input, params)
        .unwrap()
        .render(params, BTreeMap::new())
        .unwrap()
    {
        SimulaceResponse::Slovnik(body) => serde_json::to_value(body).unwrap(),
        _ => panic!("Expected dictionary"),
    }
}
fn compare(expected: &Value, actual: &Value, path: &str) {
    match expected {
        Value::Number(n) => assert!(
            (n.as_f64().unwrap() - actual.as_f64().unwrap()).abs() < 1e-8,
            "{path}: expected {expected}, got {actual}"
        ),
        Value::Object(map) => {
            for (k, v) in map {
                compare(v, &actual[k], &format!("{path}.{k}"));
            }
        }
        Value::Array(array) => {
            assert_eq!(array.len(), actual.as_array().unwrap().len(), "{path}");
            for (i, v) in array.iter().enumerate() {
                compare(v, &actual[i], &format!("{path}[{i}]"));
            }
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}
#[test]
fn matches_python_reference_for_capacity_catchments_ties_nulls_and_all_verdicts() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/simulation_reference.json")).unwrap();
    for (i, case) in cases.as_array().unwrap().iter().enumerate() {
        let input: Input = serde_json::from_value(case["input"].clone()).unwrap();
        let params: requests::SimulaceQuery =
            serde_json::from_value(case["params"].clone()).unwrap();
        let actual = value(input, &params);
        assert_eq!(
            actual["jednotky"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>(),
            case["expected"]["jednotky"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>()
        );
        compare(&case["expected"], &actual, &format!("case {i}"));
    }
}
#[test]
fn aggregated_times_use_population_weights_and_preserve_unknown_times() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/simulation_reference.json")).unwrap();
    let mut input: Input = serde_json::from_value(cases[1]["input"].clone()).unwrap();
    input.areas.truncate(2);
    input.areas[0].population = 100.0;
    input.areas[1].population = 300.0;
    input.areas[0].children = 0.2;
    input.areas[1].children = 1.4;
    input.times = vec![
        (input.areas[0].code.clone(), "600000001".into(), Some(80.0)),
        (input.areas[0].code.clone(), "600000002".into(), None),
        (input.areas[0].code.clone(), "600000003".into(), Some(40.0)),
        (input.areas[1].code.clone(), "600000001".into(), Some(100.0)),
        (input.areas[1].code.clone(), "600000002".into(), None),
        (input.areas[1].code.clone(), "600000003".into(), Some(20.0)),
    ];
    let mut params: requests::SimulaceQuery =
        serde_json::from_value(cases[1]["params"].clone()).unwrap();
    params.redizo = requests::Redizo("600000003".into());
    params.uroven = requests::Uroven::Obec;
    let body = value(input.clone(), &params);
    assert_eq!(body["jednotky"]["555738"]["cas_min_puvodni"], 95.0);
    assert_eq!(body["jednotky"]["555738"]["cas_min"], 25.0);
    assert_eq!(body["jednotky"]["555738"]["zlepseni_min"], 70.0);
    assert_eq!(body["jednotky"]["555738"]["deti"], 1.6);
    assert_eq!(body["souhrn"]["jednotek_celkem"], 1);
    params.uroven = requests::Uroven::Orp;
    assert_eq!(
        value(input.clone(), &params)["jednotky"]["4101"]["cas_min"],
        25.0
    );
    for time in &mut input.times {
        if time.1 != "600000003" {
            time.2 = None;
        }
    }
    let unknown = value(input.clone(), &params);
    assert!(unknown["jednotky"]["4101"]["cas_min_puvodni"].is_null());
    assert!(unknown["jednotky"]["4101"]["zlepseni_min"].is_null());
    input.areas.iter_mut().for_each(|a| a.population = 0.0);
    assert!(value(input, &params)["jednotky"]["4101"]["cas_min"].is_null());
}
#[test]
fn missing_matrix_data_is_an_error_and_newly_reachable_times_remain_nullable() {
    let cases: Value =
        serde_json::from_str(include_str!("fixtures/simulation_reference.json")).unwrap();
    let mut input: Input = serde_json::from_value(cases[1]["input"].clone()).unwrap();
    let params: requests::SimulaceQuery =
        serde_json::from_value(cases[1]["params"].clone()).unwrap();
    input.times.clear();
    assert!(simulation::calculate(input, &params).is_err());
    let mut area = json!({"nazev":"New reach","cas_ke_skole":30.125,"cas_min":30.125,"cas_min_puvodni":null,"zlepseni_min":null,"pasmo":"30_45","deti":1.4,"potencialni_uchazeci":0.3,"novy_dosah":true});
    assert!(serde_json::from_value::<dto::SimulacePlocha>(area.clone()).is_ok());
    area.as_object_mut().unwrap().remove("zlepseni_min");
    assert!(serde_json::from_value::<dto::SimulacePlocha>(area).is_err());
}
