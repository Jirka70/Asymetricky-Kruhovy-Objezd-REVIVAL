//! These are structural contract tests, not tests of unimplemented business calculations.
use axum::{
    Json,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    response::IntoResponse,
};
use obor_backend::{
    contract::{self, GeoJson, OPERATIONS, SPEC, SimulaceResponse},
    dto,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use tower::ServiceExt;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/openapi_responses.json")).unwrap()
}
fn response_schema(path: &str, status: &str, media: &str) -> Value {
    let response = contract::resolve(
        &SPEC["paths"][path][OPERATIONS.iter().find(|op| op.path == path).unwrap().method]["responses"]
            [status],
    );
    let schema = &response["content"][media]["schema"];
    assert!(
        !schema.is_null(),
        "Undocumented response {path}: {status} {media}"
    );
    schema.clone()
}
fn assert_valid(schema: &Value, body: &Value) {
    let validator = contract::schema_validator(schema);
    let errors: Vec<_> = validator
        .iter_errors(body)
        .map(|error| error.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "Contract violations: {}",
        errors.join("; ")
    );
}
async fn request(uri: &str, method: Method) -> axum::response::Response {
    // No connection is opened for validation, docs, or analytical stub tests.
    let pool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .connection_timeout(std::time::Duration::from_millis(50))
        .build_unchecked(
            diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(
                "postgres://localhost:1/unused_contract_test",
            ),
        );
    contract::router(pool)
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap()).unwrap()
}
async fn check_error(uri: &str, path: &str, status: StatusCode, field: Option<&str>) {
    let response = request(uri, Method::GET).await;
    assert_eq!(response.status(), status, "{uri}");
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    let body = json_body(response).await;
    assert_valid(
        &response_schema(path, status.as_str(), "application/json"),
        &body,
    );
    if let Some(field) = field {
        assert_eq!(body["error"]["pole"], field);
    }
}

#[test]
fn operation_and_success_fixture_coverage_matches_spec() {
    assert_eq!(SPEC["openapi"], "3.1.0");
    let mut specified = BTreeSet::new();
    let mut success_schemas = BTreeSet::new();
    for (path, item) in SPEC["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            if ![
                "get", "post", "put", "patch", "delete", "options", "head", "trace",
            ]
            .contains(&method.as_str())
            {
                continue;
            }
            // Every operation must have route registration and contract tests.
            specified.insert((
                path.clone(),
                method.clone(),
                operation["operationId"].as_str().unwrap().to_owned(),
            ));
            for media in operation["responses"]["200"]["content"]
                .as_object()
                .unwrap()
                .keys()
            {
                success_schemas.insert((path.clone(), media.clone()));
            }
        }
    }
    let implemented: BTreeSet<_> = OPERATIONS
        .iter()
        .map(|operation| {
            (
                operation.path.to_owned(),
                operation.method.to_owned(),
                operation.id.to_owned(),
            )
        })
        .collect();
    assert_eq!(
        implemented.len(),
        OPERATIONS.len(),
        "Duplicate operation registration"
    );
    assert_eq!(specified, implemented);
    let samples: BTreeSet<_> = fixtures()
        .iter()
        .map(|fixture| {
            (
                fixture["path"].as_str().unwrap().to_owned(),
                fixture["media_type"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        samples, success_schemas,
        "Each success response/media type needs an independent fixture"
    );
}

#[tokio::test]
async fn analytical_stubs_return_documented_501_and_all_routes_reject_wrong_method() {
    for fixture in fixtures() {
        let uri = fixture["request"].as_str().unwrap();
        let path = fixture["path"].as_str().unwrap();
        if !matches!(
            path,
            "/skoly"
                | "/skoly/{redizo}"
                | "/zsj/seznam"
                | "/obory"
                | "/obory/{kod}"
                | "/simulace"
                | "/simulace/zmeny"
                | "/student/skoly"
                | "/student/trasa"
                | "/obory/{kod}/zamestnavatele"
        ) {
            check_error(uri, path, StatusCode::NOT_IMPLEMENTED, None).await;
        }
        assert_eq!(
            request(
                uri,
                if path == "/simulace/zmeny" {
                    Method::GET
                } else {
                    Method::POST
                }
            )
            .await
            .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    assert_eq!(
        request("/api/v1/unknown", Method::GET).await.status(),
        StatusCode::NOT_FOUND
    );
}

async fn dto_response<T: DeserializeOwned + Serialize>(
    body: Value,
    geojson: bool,
) -> axum::response::Response {
    let typed: T =
        serde_json::from_value(body).expect("Fixture must deserialize into the Rust API DTO");
    if geojson {
        GeoJson(typed).into_response()
    } else {
        Json(typed).into_response()
    }
}

#[tokio::test]
async fn rust_success_dtos_and_media_types_match_openapi() {
    for fixture in fixtures() {
        let path = fixture["path"].as_str().unwrap();
        let media = fixture["media_type"].as_str().unwrap();
        let body = fixture["body"].clone();
        let schema = response_schema(path, "200", media);
        // Validate BEFORE serde so missing/invalid data cannot be hidden by coercion or omitted fields.
        assert_valid(&schema, &body);
        let geo = media == "application/geo+json";
        let response = match path {
            "/skoly" => dto_response::<dto::SkolyFeatureCollection>(body, geo).await,
            "/skoly/{redizo}" => dto_response::<dto::SkolaDetail>(body, geo).await,
            "/student/skoly" => dto_response::<dto::StudentSkoly>(body, geo).await,
            "/student/trasa" => dto_response::<dto::Trasa>(body, geo).await,
            "/zsj/seznam" => dto_response::<Vec<dto::ZsjZaznam>>(body, geo).await,
            "/zsj" => dto_response::<dto::MapaDosahu>(body, geo).await,
            "/obory" => dto_response::<dto::Obory>(body, geo).await,
            "/obory/{kod}" => dto_response::<dto::DetailOboru>(body, geo).await,
            "/obory/{kod}/zamestnavatele" => {
                dto_response::<dto::ZamestnavateleOboru>(body, geo).await
            }
            "/simulace" if geo => {
                SimulaceResponse::Geojson(serde_json::from_value(body).unwrap()).into_response()
            }
            "/simulace" => {
                SimulaceResponse::Slovnik(serde_json::from_value(body).unwrap()).into_response()
            }
            "/simulace/zmeny" if geo => dto_response::<dto::BatchSimulaceGeojson>(body, geo).await,
            "/simulace/zmeny" => dto_response::<dto::BatchSimulace>(body, geo).await,
            _ => panic!("Add a Rust DTO contract test for {path}"),
        };
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], media);
        assert_valid(&schema, &json_body(response).await);
    }
}

#[tokio::test]
async fn invalid_parameters_return_schema_compliant_422() {
    for (uri, path, field) in [
        ("/api/v1/student/skoly", "/student/skoly", "lat"),
        ("/api/v1/student/skoly?lat=50.2", "/student/skoly", "lon"),
        (
            "/api/v1/student/skoly?lat=abc&lon=12.8",
            "/student/skoly",
            "lat",
        ),
        (
            "/api/v1/student/skoly?lat=49.8&lon=12.8",
            "/student/skoly",
            "lat",
        ),
        (
            "/api/v1/student/skoly?lat=50.6&lon=12.8",
            "/student/skoly",
            "lat",
        ),
        (
            "/api/v1/student/skoly?lat=50.2&lon=13.5",
            "/student/skoly",
            "lon",
        ),
        (
            "/api/v1/student/skoly?lat=NaN&lon=12.8",
            "/student/skoly",
            "lat",
        ),
        ("/api/v1/skoly?stupen=Z", "/skoly", "stupen"),
        ("/api/v1/skoly?forma=other", "/skoly", "forma"),
        ("/api/v1/skoly?obor=invalid", "/skoly", "obor"),
        ("/api/v1/skoly/123", "/skoly/{redizo}", "redizo"),
        (
            "/api/v1/skoly/600008975?max_min=9",
            "/skoly/{redizo}",
            "max_min",
        ),
        (
            "/api/v1/skoly/600008975?max_min=181",
            "/skoly/{redizo}",
            "max_min",
        ),
        (
            "/api/v1/skoly/600008975?max_min=60.5",
            "/skoly/{redizo}",
            "max_min",
        ),
        ("/api/v1/obory?razeni=other", "/obory", "razeni"),
        ("/api/v1/obory/invalid", "/obory/{kod}", "kod"),
        (
            "/api/v1/obory/23-68-H%2F01?kandidatu=35",
            "/obory/{kod}",
            "kandidatu",
        ),
        (
            "/api/v1/student/trasa?lat=50.2&lon=12.8",
            "/student/trasa",
            "redizo",
        ),
        (
            "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01",
            "/simulace",
            "kapacita",
        ),
        (
            "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=0",
            "/simulace",
            "kapacita",
        ),
        (
            "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=301",
            "/simulace",
            "kapacita",
        ),
        (
            "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=24&format=xml",
            "/simulace",
            "format",
        ),
        ("/api/v1/skoly?forma=den&forma=dal", "/skoly", "forma"),
    ] {
        check_error(uri, path, StatusCode::UNPROCESSABLE_ENTITY, Some(field)).await;
    }
    // Inclusive bounds and an encoded slash must survive extraction.
    for uri in [
        "/api/v1/student/skoly?lat=49.9&lon=12.0&max_min=10",
        "/api/v1/student/skoly?lat=50.5&lon=13.4&max_min=180",
    ] {
        check_error(uri, "/student/skoly", StatusCode::SERVICE_UNAVAILABLE, None).await;
    }
}

#[test]
fn schema_validator_rejects_real_contract_drift() {
    let samples = fixtures();
    let school = samples
        .iter()
        .find(|fixture| fixture["path"] == "/skoly/{redizo}")
        .unwrap();
    let schema = response_schema("/skoly/{redizo}", "200", "application/json");
    let validator = contract::schema_validator(&schema);
    for field in ["redizo", "nazev", "lat", "lon", "nabidky", "spadovost"] {
        let mut body = school["body"].clone();
        body.as_object_mut().unwrap().remove(field);
        assert!(
            !validator.is_valid(&body),
            "Missing required field {field} must fail"
        );
    }
    for (field, invalid) in [
        ("redizo", json!(600008975)),
        ("redizo", json!("123")),
        ("lat", json!("50.2")),
        ("nazev", Value::Null),
        ("web", json!("not a URI")),
    ] {
        let mut body = school["body"].clone();
        body[field] = invalid;
        assert!(!validator.is_valid(&body), "Invalid {field} must fail");
    }
    let mut point = json!({"type":"Point","coordinates":[12.8,50.2]});
    let geometry_validator = contract::schema_validator(&SPEC["components"]["schemas"]["GeoPoint"]);
    assert!(geometry_validator.is_valid(&point));
    point["coordinates"] = json!([12.8]);
    assert!(!geometry_validator.is_valid(&point));
    point["coordinates"] = json!([12.8, 50.2, 0]);
    assert!(!geometry_validator.is_valid(&point));
    point["coordinates"] = json!([12.8, 50.2]);
    point["type"] = json!("LineString");
    assert!(!geometry_validator.is_valid(&point));
    let simulation = samples
        .iter()
        .find(|fixture| {
            fixture["path"] == "/simulace" && fixture["media_type"] == "application/json"
        })
        .unwrap();
    let validator =
        contract::schema_validator(&response_schema("/simulace", "200", "application/json"));
    let mut body = simulation["body"].clone();
    let value = body["jednotky"]
        .as_object_mut()
        .unwrap()
        .remove("000540")
        .unwrap();
    body["jednotky"]["540"] = value;
    assert!(
        !validator.is_valid(&body),
        "Dictionary keys must retain 4–6 digit area codes"
    );
    let trasa = samples
        .iter()
        .find(|fixture| fixture["path"] == "/student/trasa")
        .unwrap();
    let validator = contract::schema_validator(&response_schema(
        "/student/trasa",
        "200",
        "application/geo+json",
    ));
    let mut body = trasa["body"].clone();
    body["meta"]["den"] = json!("2026-99-99");
    assert!(!validator.is_valid(&body));
    let mut body = trasa["body"].clone();
    body["spoje"] = json!(vec![body["spoje"][0].clone(); 4]);
    assert!(!validator.is_valid(&body));
    let mut body = trasa["body"].clone();
    body["features"][0]["properties"]["druh"] = json!("PLANE");
    assert!(!validator.is_valid(&body));
}

#[test]
fn zsj_list_requires_polygon_and_preserves_nullable_municipality() {
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture["path"] == "/zsj/seznam")
        .unwrap();
    let schema = response_schema("/zsj/seznam", "200", "application/json");
    let validator = contract::schema_validator(&schema);
    assert_valid(&schema, &json!([]));
    let mut body = fixture["body"].clone();
    body[0]["kod_obce"] = Value::Null;
    assert_valid(&schema, &body);
    let rows: Vec<dto::ZsjZaznam> = serde_json::from_value(body.clone()).unwrap();
    assert!(rows[0].kod_obce.is_none());
    assert_valid(&schema, &serde_json::to_value(rows).unwrap());
    for field in ["boundary", "kod_obce"] {
        let mut missing = body.clone();
        missing[0].as_object_mut().unwrap().remove(field);
        assert!(!validator.is_valid(&missing));
        assert!(serde_json::from_value::<Vec<dto::ZsjZaznam>>(missing).is_err());
    }
    body[0]["boundary"]["type"] = json!("Point");
    assert!(!validator.is_valid(&body));
    assert!(serde_json::from_value::<Vec<dto::ZsjZaznam>>(body).is_err());
}

#[test]
fn every_component_and_declared_response_schema_compiles() {
    for schema in SPEC["components"]["schemas"].as_object().unwrap().values() {
        contract::schema_validator(schema);
    }
    for operation in OPERATIONS {
        for response in SPEC["paths"][operation.path][operation.method]["responses"]
            .as_object()
            .unwrap()
            .values()
        {
            for media in contract::resolve(response)["content"]
                .as_object()
                .unwrap()
                .values()
            {
                contract::schema_validator(&media["schema"]);
            }
        }
    }
}

#[tokio::test]
async fn served_documentation_uses_single_specification() {
    let response = request("/openapi.yaml", Method::GET).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/yaml");
    let body = to_bytes(response.into_body(), 2_000_000).await.unwrap();
    assert_eq!(body.as_ref(), contract::SPEC_YAML.as_bytes());
    let response = request("/docs", Method::GET).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 100_000).await.unwrap();
    let html = std::str::from_utf8(&body).unwrap();
    assert!(html.contains("url: './openapi.yaml'"));
    assert!(
        !html.contains("const spec ="),
        "Do not embed a second copy of the specification"
    );
}

// Compare all declared DTO fields, including optional fields not exercised by a fixture.
// Value constraints (patterns, bounds, formats, nullability) are enforced by the YAML
// validator rather than pretending String/Vec Rust types encode those constraints.
fn dto_shape(schema: &Value, root: &Value) -> Value {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        return dto_shape(
            root.pointer(reference.strip_prefix('#').unwrap()).unwrap(),
            root,
        );
    }
    if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
        let mut properties = serde_json::Map::new();
        let mut required = BTreeSet::new();
        for part in parts {
            let shape = dto_shape(part, root);
            assert_eq!(
                shape["type"], "object",
                "Only object intersections are used in this contract"
            );
            properties.extend(shape["properties"].as_object().unwrap().clone());
            required.extend(
                shape["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_owned()),
            );
        }
        return json!({"type":"object", "properties":properties, "required":required});
    }
    for union in ["oneOf", "anyOf"] {
        if let Some(parts) = schema.get(union).and_then(Value::as_array) {
            let nonnull: Vec<_> = parts.iter().filter(|part| part["type"] != "null").collect();
            assert_eq!(
                nonnull.len(),
                1,
                "Add shape comparison for new non-null unions"
            );
            return dto_shape(nonnull[0], root);
        }
    }
    let kind = match &schema["type"] {
        Value::String(kind) => kind.as_str(),
        Value::Array(kinds) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|kind| *kind != "null")
            .unwrap(),
        _ => "any",
    };
    let mut shape = json!({"type":kind});
    match kind {
        "object" => {
            let properties: serde_json::Map<_, _> = schema["properties"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, field)| (name.clone(), dto_shape(field, root)))
                .collect();
            let required: BTreeSet<_> = schema["required"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|v| v.as_str().unwrap())
                .collect();
            shape["properties"] = Value::Object(properties);
            shape["required"] = json!(required);
            if schema["additionalProperties"].is_object() {
                shape["values"] = dto_shape(&schema["additionalProperties"], root);
            }
        }
        "array" => shape["items"] = dto_shape(&schema["items"], root),
        _ => {}
    }
    if let Some(value) = schema.get("const") {
        shape["enum"] = json!([value]);
    } else if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let mut sorted = values.clone();
        sorted.sort_by_key(Value::to_string);
        shape["enum"] = json!(sorted);
    }
    shape
}

fn check_dto_shape<T: schemars::JsonSchema>(schema: Value) {
    let rust_schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    assert_eq!(
        dto_shape(&schema, &SPEC),
        dto_shape(&rust_schema, &rust_schema),
        "Rust DTO structure differs from openapi.yaml: {}",
        std::any::type_name::<T>()
    );
}

#[test]
fn all_response_dto_fields_types_enums_and_required_fields_match_spec() {
    check_dto_shape::<dto::SkolyFeatureCollection>(response_schema(
        "/skoly",
        "200",
        "application/geo+json",
    ));
    check_dto_shape::<dto::SkolaDetail>(response_schema(
        "/skoly/{redizo}",
        "200",
        "application/json",
    ));
    check_dto_shape::<dto::StudentSkoly>(response_schema(
        "/student/skoly",
        "200",
        "application/json",
    ));
    check_dto_shape::<dto::Trasa>(response_schema(
        "/student/trasa",
        "200",
        "application/geo+json",
    ));
    check_dto_shape::<Vec<dto::ZsjZaznam>>(response_schema(
        "/zsj/seznam",
        "200",
        "application/json",
    ));
    check_dto_shape::<dto::MapaDosahu>(response_schema("/zsj", "200", "application/geo+json"));
    check_dto_shape::<dto::Obory>(response_schema("/obory", "200", "application/json"));
    check_dto_shape::<dto::DetailOboru>(response_schema("/obory/{kod}", "200", "application/json"));
    check_dto_shape::<dto::Simulace>(response_schema("/simulace", "200", "application/json"));
    check_dto_shape::<dto::SimulaceGeojson>(response_schema(
        "/simulace",
        "200",
        "application/geo+json",
    ));
    check_dto_shape::<dto::ZamestnavateleOboru>(response_schema(
        "/obory/{kod}/zamestnavatele",
        "200",
        "application/geo+json",
    ));
    check_dto_shape::<dto::BatchSimulace>(response_schema(
        "/simulace/zmeny",
        "200",
        "application/json",
    ));
    check_dto_shape::<dto::BatchSimulaceGeojson>(response_schema(
        "/simulace/zmeny",
        "200",
        "application/geo+json",
    ));
    check_dto_shape::<obor_backend::requests::BatchSimulaceRequest>(
        SPEC["components"]["schemas"]["BatchSimulaceRequest"].clone(),
    );
    check_dto_shape::<dto::Chyba>(SPEC["components"]["schemas"]["Chyba"].clone());
}

fn check_query_schema<T: obor_backend::requests::RequestQuery + schemars::JsonSchema>() {
    let operation = OPERATIONS
        .iter()
        .find(|operation| operation.id == T::OPERATION_ID)
        .unwrap();
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for parameter in SPEC["paths"][operation.path]["get"]["parameters"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let parameter = contract::resolve(parameter);
        if parameter["in"] != "query" {
            continue;
        }
        let name = parameter["name"].as_str().unwrap();
        properties.insert(name.to_owned(), parameter["schema"].clone());
        if parameter["required"].as_bool().unwrap_or(false) {
            required.push(name);
        }
    }
    let expected = json!({"type":"object", "properties":properties, "required":required});
    check_dto_shape::<T>(expected.clone());
    let rust_schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    for (name, schema) in expected["properties"].as_object().unwrap() {
        let schema = contract::resolve(schema);
        if let Some(default) = schema.get("default") {
            assert_eq!(
                &rust_schema["properties"][name]["default"],
                default,
                "Default for {}.{name} differs from OpenAPI",
                T::OPERATION_ID
            );
        }
    }
}

#[test]
fn every_query_struct_matches_declared_fields_types_enums_required_and_defaults() {
    use obor_backend::requests::*;
    check_query_schema::<SkolyQuery>();
    check_query_schema::<SkolaQuery>();
    check_query_schema::<StudentSkolyQuery>();
    check_query_schema::<StudentTrasaQuery>();
    check_query_schema::<ZsjSeznamQuery>();
    check_query_schema::<ZsjQuery>();
    check_query_schema::<OboryQuery>();
    check_query_schema::<OborQuery>();
    check_query_schema::<SimulaceQuery>();
    check_query_schema::<OborZamestnavateleQuery>();
}

async fn extract_query<T: obor_backend::requests::RequestQuery>(uri: &str) -> T {
    use axum::extract::FromRequestParts;
    let (mut parts, _) = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap()
        .into_parts();
    contract::ContractQuery::<T>::from_request_parts(&mut parts, &())
        .await
        .unwrap()
        .0
}

#[tokio::test]
async fn query_extractor_applies_defaults_and_exposes_typed_filters_and_identifiers() {
    use obor_backend::requests::*;
    let schools =
        extract_query::<SkolyQuery>("/api/v1/skoly?obor=23-68-H%2F01&stupen=H,M&forma=den").await;
    assert_eq!(schools.obor.unwrap(), KodOboru("23-68-H/01".into()));
    assert_eq!(
        schools.stupen.unwrap().0,
        vec![dto::Stupen::H, dto::Stupen::M]
    );
    assert_eq!(schools.forma, Forma::Den);
    let school = extract_query::<SkolaQuery>("/api/v1/skoly/600008975").await;
    assert_eq!(school.max_min, 120);
    assert_eq!(school.scenar, Scenar::Rano);
    let student =
        extract_query::<StudentSkolyQuery>("/api/v1/student/skoly?lat=50.2312&lon=12.8711").await;
    assert_eq!((student.lat, student.lon), (50.2312, 12.8711));
    assert_eq!(student.max_min, 120);
    assert_eq!(student.scenar, Scenar::Rano);
    assert!(student.obor.is_none());
    assert_eq!(student.forma, Forma::Den);
    let route = extract_query::<StudentTrasaQuery>(
        "/api/v1/student/trasa?lat=50.2&lon=12.8&redizo=000008975",
    )
    .await;
    assert_eq!(route.redizo, Redizo("000008975".into()));
    assert_eq!(route.scenar, Scenar::Rano);
    let municipalities =
        extract_query::<ZsjQuery>("/api/v1/zsj?max_min=180&uroven=orp&forma=dal").await;
    assert_eq!(municipalities.max_min, 180);
    assert_eq!(municipalities.scenar, Scenar::Rano);
    assert_eq!(municipalities.uroven, Uroven::Orp);
    assert_eq!(municipalities.forma, Forma::Dal);
    let programs = extract_query::<OboryQuery>("/api/v1/obory?signal=pretlak,poptavka_trhu").await;
    assert_eq!(programs.razeni, Razeni::Nazev);
    assert_eq!(programs.signal.unwrap().0, vec!["pretlak", "poptavka_trhu"]);
    let programs = extract_query::<OboryQuery>("/api/v1/obory?razeni=-index_pretlaku").await;
    assert_eq!(programs.razeni, Razeni::IndexPretlakuSestupne);
    let program = extract_query::<OborQuery>("/api/v1/obory/23-68-H%2F01").await;
    assert_eq!(program.kandidatu, 5);
    assert_eq!(program.max_min, 120);
    let uri = "/api/v1/simulace?redizo=000008975&obor=23-68-H%2F01&kapacita=24";
    let simulation = extract_query::<SimulaceQuery>(uri).await;
    assert_eq!(simulation.redizo, Redizo("000008975".into()));
    assert_eq!(simulation.obor, KodOboru("23-68-H/01".into()));
    assert_eq!(simulation.kapacita, 24);
    assert_eq!(simulation.max_min, 120);
    assert_eq!(simulation.scenar, Scenar::Rano);
    assert_eq!(simulation.format, Format::Slovnik);
    assert_eq!(simulation.uroven, Uroven::Obec);
    let simulation =
        extract_query::<SimulaceQuery>(&format!("{uri}&format=geojson&max_min=90")).await;
    assert_eq!(simulation.format, Format::Geojson);
    assert_eq!(simulation.max_min, 90);
}

#[tokio::test]
async fn every_typed_query_serializes_values_accepted_by_openapi() {
    use obor_backend::requests::*;
    async fn check<T: RequestQuery + Serialize>(fixture: &Value) {
        let typed = extract_query::<T>(fixture["request"].as_str().unwrap()).await;
        let body = serde_json::to_value(typed).unwrap();
        let path = fixture["path"].as_str().unwrap();
        for parameter in SPEC["paths"][path]["get"]["parameters"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let parameter = contract::resolve(parameter);
            if parameter["in"] != "query" {
                continue;
            }
            let name = parameter["name"].as_str().unwrap();
            if let Some(value) = body.get(name) {
                assert_valid(&parameter["schema"], value);
            } else {
                assert!(
                    !parameter["required"].as_bool().unwrap_or(false),
                    "Missing required typed field {name}"
                );
            }
        }
    }
    for fixture in fixtures() {
        match fixture["path"].as_str().unwrap() {
            "/skoly" => check::<SkolyQuery>(&fixture).await,
            "/skoly/{redizo}" => check::<SkolaQuery>(&fixture).await,
            "/student/skoly" => check::<StudentSkolyQuery>(&fixture).await,
            "/student/trasa" => check::<StudentTrasaQuery>(&fixture).await,
            "/zsj/seznam" => check::<ZsjSeznamQuery>(&fixture).await,
            "/zsj" => check::<ZsjQuery>(&fixture).await,
            "/obory" => check::<OboryQuery>(&fixture).await,
            "/obory/{kod}" => check::<OborQuery>(&fixture).await,
            "/simulace" => check::<SimulaceQuery>(&fixture).await,
            "/obory/{kod}/zamestnavatele" => check::<OborZamestnavateleQuery>(&fixture).await,
            "/simulace/zmeny" => continue,
            path => panic!("Missing typed query test for {path}"),
        }
    }
}

#[tokio::test]
async fn new_enum_and_employer_parameters_validate_and_apply_defaults() {
    use obor_backend::{requests::*, types::Vhodnost};
    let uri = "/api/v1/obory/65-51-H%2F01/zamestnavatele";
    let employers = extract_query::<OborZamestnavateleQuery>(uri).await;
    assert_eq!(employers.vhodnost, Vhodnost::Vhodne);
    assert!(employers.jen_ss);
    let employers =
        extract_query::<OborZamestnavateleQuery>(&format!("{uri}?vhodnost=1&jen_ss=false")).await;
    assert_eq!(employers.vhodnost, Vhodnost::Nejvhodnejsi);
    assert!(!employers.jen_ss);
    let map = extract_query::<ZsjQuery>("/api/v1/zsj").await;
    assert_eq!(map.uroven, Uroven::Obec);
    assert_eq!(map.forma, Forma::Den);
    assert_eq!(map.max_min, 120);
    for level in ["orp", "obec"] {
        check_error(
            &format!("/api/v1/zsj?uroven={level}&max_min=180"),
            "/zsj",
            StatusCode::NOT_IMPLEMENTED,
            None,
        )
        .await;
    }
    for (uri, path, field) in [
        ("/api/v1/zsj?uroven=okres", "/zsj", "uroven"),
        ("/api/v1/zsj?scenar=odpoledne", "/zsj", "scenar"),
        ("/api/v1/obory?forma=other", "/obory", "forma"),
        (
            "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=24&uroven=other",
            "/simulace",
            "uroven",
        ),
        (
            "/api/v1/obory/invalid/zamestnavatele",
            "/obory/{kod}/zamestnavatele",
            "kod",
        ),
        (
            "/api/v1/obory/65-51-H%2F01/zamestnavatele?vhodnost=0",
            "/obory/{kod}/zamestnavatele",
            "vhodnost",
        ),
        (
            "/api/v1/obory/65-51-H%2F01/zamestnavatele?vhodnost=3",
            "/obory/{kod}/zamestnavatele",
            "vhodnost",
        ),
        (
            "/api/v1/obory/65-51-H%2F01/zamestnavatele?jen_ss=1",
            "/obory/{kod}/zamestnavatele",
            "jen_ss",
        ),
        (
            "/api/v1/obory/65-51-H%2F01/zamestnavatele?jen_ss=maybe",
            "/obory/{kod}/zamestnavatele",
            "jen_ss",
        ),
    ] {
        check_error(uri, path, StatusCode::UNPROCESSABLE_ENTITY, Some(field)).await;
    }
    for uri in [
        "/health",
        "/api/skoly",
        "/api/skoly/600008975",
        "/api/zsj",
        "/api/zsj/000540",
        "/api/obory",
        "/api/nabidky",
        "/api/zamestnavatele",
        "/api/profesni-skupiny",
        "/api/poptavka-profesi",
        "/api/obor-profese",
        "/api/v1/ciselniky",
        "/api/v1/analyza/obce",
        "/api/v1/analyza/obory",
        "/api/v1/analyza/obory/23-68-H%2F01",
        "/api/v1/analyza/simulace",
    ] {
        assert_eq!(
            request(uri, Method::GET).await.status(),
            StatusCode::NOT_FOUND,
            "Removed route {uri}"
        );
    }
}

#[test]
fn employer_ids_numeric_enums_and_required_nulls_match_new_contract() {
    use obor_backend::types::Vhodnost;
    assert_eq!(
        serde_json::to_value(Vhodnost::Nejvhodnejsi).unwrap(),
        json!(1)
    );
    assert_eq!(serde_json::to_value(Vhodnost::Vhodne).unwrap(), json!(2));
    assert!(serde_json::from_value::<Vhodnost>(json!(3)).is_err());
    let sample = fixtures()
        .into_iter()
        .find(|fixture| fixture["path"] == "/obory/{kod}/zamestnavatele")
        .unwrap();
    let schema = response_schema("/obory/{kod}/zamestnavatele", "200", "application/geo+json");
    let validator = contract::schema_validator(&schema);
    let mut body = sample["body"].clone();
    body["features"] = json!([]);
    body["meta"] = json!({"obor":"65-51-H/01", "mapovani":false, "existuje":null, "zamestnavatelu":null, "pracovist":null, "pocet_mist":null});
    assert_valid(&schema, &body);
    let typed: dto::ZamestnavateleOboru = serde_json::from_value(body.clone()).unwrap();
    let serialized = serde_json::to_value(typed).unwrap();
    assert!(
        serialized["meta"]
            .as_object()
            .unwrap()
            .contains_key("existuje")
    );
    assert!(serialized["meta"]["existuje"].is_null());
    assert_valid(&schema, &serialized);
    body["meta"].as_object_mut().unwrap().remove("existuje");
    assert!(!validator.is_valid(&body));
    assert!(serde_json::from_value::<dto::ZamestnavateleOboru>(body).is_err());
    let mut body = sample["body"].clone();
    body["features"][0]["properties"]["id"] = json!(84980608601418621_u64);
    assert!(
        !validator.is_valid(&body),
        "Workplace IDs must be strings for JS precision"
    );
    let mut body = sample["body"].clone();
    body["features"][0]["properties"]["ico"] = json!("123");
    assert!(!validator.is_valid(&body));
    let mut body = sample["body"].clone();
    body["features"][0]["properties"]["profese"][0]["vhodnost"] = json!(3);
    assert!(!validator.is_valid(&body));
}

#[test]
fn simulation_area_codes_bands_and_required_nullable_times_match_new_contract() {
    let sample = fixtures()
        .into_iter()
        .find(|fixture| {
            fixture["path"] == "/simulace" && fixture["media_type"] == "application/json"
        })
        .unwrap();
    let schema = response_schema("/simulace", "200", "application/json");
    let mut body = sample["body"].clone();
    let area = body["jednotky"]
        .as_object_mut()
        .unwrap()
        .remove("000540")
        .unwrap();
    body["jednotky"]["4101"] = area;
    body["meta"]["uroven"] = json!("orp");
    body["jednotky"]["4101"]["cas_ke_skole"] = Value::Null;
    body["jednotky"]["4101"]["cas_min"] = Value::Null;
    body["jednotky"]["4101"]["pasmo"] = json!("bez_spojeni");
    assert_valid(&schema, &body);
    let typed: dto::Simulace = serde_json::from_value(body.clone()).unwrap();
    let serialized = serde_json::to_value(typed).unwrap();
    assert!(
        serialized["jednotky"]["4101"]
            .as_object()
            .unwrap()
            .contains_key("cas_min")
    );
    assert!(serialized["jednotky"]["4101"]["cas_min"].is_null());
    assert_valid(&schema, &serialized);
    body["jednotky"]["4101"]
        .as_object_mut()
        .unwrap()
        .remove("cas_min");
    assert!(!contract::schema_validator(&schema).is_valid(&body));
    assert!(serde_json::from_value::<dto::Simulace>(body).is_err());
    for band in [
        "do30",
        "30_45",
        "45_60",
        "nad60",
        "mimo_dosah",
        "bez_spojeni",
    ] {
        let typed: dto::Pasmo = serde_json::from_value(json!(band)).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), json!(band));
    }
    assert!(serde_json::from_value::<dto::Pasmo>(json!(3045)).is_err());
}

#[tokio::test]
async fn database_read_routes_return_documented_503_when_pool_is_unavailable() {
    for (uri, path) in [
        ("/api/v1/skoly", "/skoly"),
        (
            "/api/v1/student/skoly?lat=50.2312&lon=12.8711",
            "/student/skoly",
        ),
        (
            "/api/v1/student/trasa?lat=50.2312&lon=12.8711&redizo=600009084",
            "/student/trasa",
        ),
        ("/api/v1/zsj/seznam", "/zsj/seznam"),
        ("/api/v1/zsj?uroven=zsj", "/zsj"),
        ("/api/v1/obory", "/obory"),
        (
            "/api/v1/simulace?redizo=600009271&obor=23-68-H%2F01&kapacita=30",
            "/simulace",
        ),
        ("/api/v1/obory/65-51-H%2F01", "/obory/{kod}"),
        ("/api/v1/skoly/600008975", "/skoly/{redizo}"),
        (
            "/api/v1/obory/65-51-H%2F01/zamestnavatele",
            "/obory/{kod}/zamestnavatele",
        ),
    ] {
        check_error(uri, path, StatusCode::SERVICE_UNAVAILABLE, None).await;
    }
}

#[tokio::test]
async fn program_catalog_ignores_travel_parameters_and_validates_catalog_filters() {
    use obor_backend::requests::*;
    let query = extract_query::<OboryQuery>(
        "/api/v1/obory?max_min=invalid&max_min=-1&scenar=anything&scenar=other&stupen=H,M&forma=dal",
    ).await;
    assert_eq!(query.forma, Forma::Dal);
    assert_eq!(
        query.stupen.unwrap().0,
        vec![dto::Stupen::H, dto::Stupen::M]
    );
    for uri in [
        "/api/v1/obory?signal=spatna_dostupnost",
        "/api/v1/obory?signal=unknown",
        "/api/v1/obory?signal=",
        "/api/v1/obory?signal=pretlak,unknown",
    ] {
        check_error(
            uri,
            "/obory",
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("signal"),
        )
        .await;
    }
    for uri in [
        "/api/v1/obory?razeni=deti_bez_oboru",
        "/api/v1/obory?razeni=mist_na_100_deti",
    ] {
        check_error(
            uri,
            "/obory",
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("razeni"),
        )
        .await;
    }
    check_error(
        "/api/v1/obory?forma=den&forma=dal",
        "/obory",
        StatusCode::UNPROCESSABLE_ENTITY,
        Some("forma"),
    )
    .await;
}

#[tokio::test]
async fn student_school_endpoint_validates_travel_parameters() {
    for (uri, field) in [
        (
            "/api/v1/student/skoly?lat=50.2&lon=12.8&max_min=9",
            "max_min",
        ),
        (
            "/api/v1/student/skoly?lat=50.2&lon=12.8&max_min=181",
            "max_min",
        ),
        (
            "/api/v1/student/skoly?lat=50.2&lon=12.8&max_min=invalid",
            "max_min",
        ),
        (
            "/api/v1/student/skoly?lat=50.2&lon=12.8&scenar=other",
            "scenar",
        ),
    ] {
        check_error(
            uri,
            "/student/skoly",
            StatusCode::UNPROCESSABLE_ENTITY,
            Some(field),
        )
        .await;
    }
}

#[tokio::test]
async fn batch_simulation_validates_json_body_before_accessing_database() {
    let pool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .connection_timeout(std::time::Duration::from_millis(50))
        .build_unchecked(
            diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(
                "postgres://localhost:1/unused_contract_test",
            ),
        );
    let app = contract::router(pool);
    let valid = json!({"obor":"23-68-H/01","zmeny":[{"redizo":"600009271","zmena_kapacity":30}]});
    let typed: obor_backend::requests::BatchSimulaceRequest =
        serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(typed.max_min, 120);
    assert_eq!(typed.uroven, obor_backend::requests::Uroven::Obec);
    assert_eq!(typed.format, obor_backend::requests::Format::Slovnik);
    assert_valid(
        &SPEC["components"]["schemas"]["BatchSimulaceRequest"],
        &serde_json::to_value(typed).unwrap(),
    );
    let mut invalids = vec![json!({}), json!({"obor":"23-68-H/01","zmeny":[]})];
    for (pointer, value) in [
        ("/obor", json!("invalid")),
        ("/zmeny/0/redizo", json!(123)),
        ("/zmeny/0/redizo", json!("abc")),
        ("/zmeny/0/zmena_kapacity", json!(0)),
        ("/zmeny/0/zmena_kapacity", json!(-301)),
        ("/zmeny/0/zmena_kapacity", json!(301)),
        ("/zmeny/0/zmena_kapacity", json!(1.5)),
        ("/zmeny", json!(vec![valid["zmeny"][0].clone(); 101])),
    ] {
        let mut v = valid.clone();
        *v.pointer_mut(pointer).unwrap() = value;
        invalids.push(v);
    }
    for (field, value) in [
        ("max_min", json!(9)),
        ("max_min", json!(181)),
        ("max_min", json!(120.5)),
        ("scenar", json!("odpoledne")),
        ("uroven", json!("invalid")),
        ("format", json!("xml")),
        ("unexpected", json!(true)),
    ] {
        let mut v = valid.clone();
        v[field] = value;
        invalids.push(v);
    }
    for (value, status) in invalids
        .into_iter()
        .map(|v| (v, StatusCode::UNPROCESSABLE_ENTITY))
        .chain([
            (valid, StatusCode::SERVICE_UNAVAILABLE),
            (json!({"obor":"23-68-H/01","max_min":120.0,"zmeny":[{"redizo":"600009271","zmena_kapacity":30.0}]}), StatusCode::SERVICE_UNAVAILABLE),
        ])
    {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/simulace/zmeny")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(value.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{value}");
        let body = json_body(response).await;
        assert_valid(
            &response_schema("/simulace/zmeny", status.as_str(), "application/json"),
            &body,
        );
        if status == StatusCode::UNPROCESSABLE_ENTITY {
            assert_eq!(body["error"]["pole"], "body");
        }
    }
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/simulace/zmeny")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[test]
fn batch_json_integer_fields_accept_decimal_and_exponent_notation_without_truncation() {
    use obor_backend::requests::BatchSimulaceRequest;
    for raw in [
        r#"{"obor":"23-68-H/01","max_min":120.0,"zmeny":[{"redizo":"600009271","zmena_kapacity":30.0},{"redizo":"600009084","zmena_kapacity":-30.0}]}"#,
        r#"{"obor":"23-68-H/01","max_min":1.2e2,"zmeny":[{"redizo":"600009271","zmena_kapacity":3e1},{"redizo":"600009084","zmena_kapacity":-3e1}]}"#,
    ] {
        let value: Value = serde_json::from_str(raw).unwrap();
        assert_valid(
            &SPEC["components"]["schemas"]["BatchSimulaceRequest"],
            &value,
        );
        let typed: BatchSimulaceRequest = serde_json::from_value(value).unwrap();
        assert_eq!(typed.max_min, 120);
        assert_eq!(typed.zmeny[0].zmena_kapacity, 30);
        assert_eq!(typed.zmeny[1].zmena_kapacity, -30);
    }
    for value in [
        json!(30.5),
        json!(-30.5),
        json!("30"),
        json!(true),
        json!(1e40),
    ] {
        assert!(
            serde_json::from_value::<BatchSimulaceRequest>(
                json!({"obor":"23-68-H/01","zmeny":[{"redizo":"600009271","zmena_kapacity":value}]})
            )
            .is_err()
        );
    }
}

#[tokio::test]
async fn invalid_utf8_path_identifiers_return_documented_json_422() {
    for invalid in ["%FF", "%C3%28", "%E2%82", "valid%FFsuffix"] {
        for (uri, path, field) in [
            (
                format!("/api/v1/skoly/{invalid}"),
                "/skoly/{redizo}",
                "redizo",
            ),
            (format!("/api/v1/obory/{invalid}"), "/obory/{kod}", "kod"),
            (
                format!("/api/v1/obory/{invalid}/zamestnavatele"),
                "/obory/{kod}/zamestnavatele",
                "kod",
            ),
        ] {
            check_error(&uri, path, StatusCode::UNPROCESSABLE_ENTITY, Some(field)).await;
        }
    }
}

#[tokio::test]
async fn zsj_list_repeated_query_parameters_return_documented_json_422() {
    for (query, field) in [
        ("x=1&x=2", "x"),
        ("max_min=10&max_min=20", "max_min"),
        ("%78=1&x=2", "x"),
    ] {
        check_error(
            &format!("/api/v1/zsj/seznam?{query}"),
            "/zsj/seznam",
            StatusCode::UNPROCESSABLE_ENTITY,
            Some(field),
        )
        .await;
    }
}
