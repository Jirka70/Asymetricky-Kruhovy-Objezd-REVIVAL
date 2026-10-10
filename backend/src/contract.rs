//! OpenAPI handlers, query validation, and stubs for analytical operations.
use crate::{
    db::DbPool,
    dto,
    requests::{self, RequestQuery},
};
use axum::{
    Extension, Json, Router,
    extract::{FromRequestParts, Path, Query, State},
    http::{StatusCode, header, request::Parts},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock};

pub const SPEC_YAML: &str = include_str!("../../openapi.yaml");
pub static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_yaml_ng::from_str(SPEC_YAML).expect("openapi.yaml must be valid YAML/JSON")
});

#[derive(Clone, Copy, Debug)]
pub struct Operation {
    pub path: &'static str,
    pub id: &'static str,
    pub method: &'static str,
}
pub const OPERATIONS: &[Operation] = &[
    Operation {
        path: "/skoly",
        id: "listSkoly",
        method: "get",
    },
    Operation {
        path: "/skoly/{redizo}",
        id: "getSkola",
        method: "get",
    },
    Operation {
        path: "/student/skoly",
        id: "listStudentSkoly",
        method: "get",
    },
    Operation {
        path: "/student/trasa",
        id: "getStudentTrasa",
        method: "get",
    },
    Operation {
        path: "/zsj/seznam",
        id: "listZsj",
        method: "get",
    },
    Operation {
        path: "/zsj",
        id: "getZsj",
        method: "get",
    },
    Operation {
        path: "/obory",
        id: "listObory",
        method: "get",
    },
    Operation {
        path: "/obory/{kod}",
        id: "getObor",
        method: "get",
    },
    Operation {
        path: "/obory/{kod}/zamestnavatele",
        id: "listOborZamestnavatele",
        method: "get",
    },
    Operation {
        path: "/simulace",
        id: "getSimulace",
        method: "get",
    },
    Operation {
        path: "/simulace/zmeny",
        id: "postSimulaceZmeny",
        method: "post",
    },
];

/// Resolve local OpenAPI references (response and parameter objects as well as schemas).
pub fn resolve(value: &Value) -> &Value {
    if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
        let pointer = reference
            .strip_prefix('#')
            .expect("Only local OpenAPI references are supported");
        resolve(SPEC.pointer(pointer).expect("Unresolved OpenAPI reference"))
    } else {
        value
    }
}

/// Keep all component references in scope and explicitly use the OpenAPI 3.1 dialect.
pub fn schema_validator(schema: &Value) -> jsonschema::Validator {
    let root = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "components": SPEC["components"],
        "allOf": [schema]
    });
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&root)
        .expect("Invalid OpenAPI JSON Schema")
}

struct Parameter {
    name: String,
    location: String,
    required: bool,
    kind: String,
    validator: jsonschema::Validator,
}
static PARAMETERS: LazyLock<BTreeMap<&'static str, Vec<Parameter>>> = LazyLock::new(|| {
    OPERATIONS
        .iter()
        .map(|operation| {
            let parameters = SPEC["paths"][operation.path][operation.method]["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|parameter| {
                    let parameter = resolve(parameter);
                    let schema = &parameter["schema"];
                    Parameter {
                        name: parameter["name"].as_str().unwrap().into(),
                        location: parameter["in"].as_str().unwrap().into(),
                        required: parameter["required"].as_bool().unwrap_or(false),
                        kind: resolve(schema)["type"].as_str().unwrap_or("string").into(),
                        validator: schema_validator(schema),
                    }
                })
                .collect();
            (operation.id, parameters)
        })
        .collect()
});

#[derive(Debug)]
pub struct StubError {
    status: StatusCode,
    body: dto::Chyba,
}
impl StubError {
    pub(crate) fn invalid(field: &str) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            body: dto::Chyba {
                error: dto::ChybaError {
                    kod: "neplatny_parametr".into(),
                    zprava: format!("Chybí nebo je neplatný parametr: {field}"),
                    pole: Some(field.into()),
                },
            },
        }
    }
    fn unimplemented(operation: &str) -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            body: dto::Chyba {
                error: dto::ChybaError {
                    kod: "neimplementovano".into(),
                    zprava: format!("Endpoint {operation} zatím není implementovaný."),
                    pole: None,
                },
            },
        }
    }
}
impl IntoResponse for StubError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

pub(crate) fn api_error(status: StatusCode, kod: &str, zprava: &str) -> StubError {
    StubError {
        status,
        body: dto::Chyba {
            error: dto::ChybaError {
                kod: kod.into(),
                zprava: zprava.into(),
                pole: None,
            },
        },
    }
}

fn validate_query(operation: &str, raw_query: Option<String>) -> Result<(), StubError> {
    let pairs: Vec<(String, String)> =
        serde_urlencoded::from_str(raw_query.as_deref().unwrap_or_default())
            .map_err(|_| StubError::invalid("query"))?;
    let mut query = BTreeMap::new();
    for (name, value) in pairs {
        if query.insert(name.clone(), value).is_some() {
            return Err(StubError::invalid(&name));
        }
    }
    for parameter in &PARAMETERS[operation] {
        if parameter.location != "query" {
            continue;
        }
        let value = query.get(&parameter.name).map(String::as_str);
        let Some(value) = value else {
            if parameter.required {
                return Err(StubError::invalid(&parameter.name));
            }
            continue;
        };
        let parsed = match parameter.kind.as_str() {
            "integer" => value.parse::<i64>().ok().map(Value::from),
            "number" => value
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(Value::from),
            "boolean" => value.parse::<bool>().ok().map(Value::from),
            _ => Some(Value::String(value.into())),
        }
        .ok_or_else(|| StubError::invalid(&parameter.name))?;
        if !parameter.validator.is_valid(&parsed) {
            return Err(StubError::invalid(&parameter.name));
        }
    }
    Ok(())
}

/// GeoJSON responses have their own media type in the specification.
pub struct GeoJson<T>(pub T);
impl<T: Serialize> IntoResponse for GeoJson<T> {
    fn into_response(self) -> Response {
        (
            [(header::CONTENT_TYPE, "application/geo+json")],
            Json(self.0),
        )
            .into_response()
    }
}
pub enum SimulaceResponse {
    Slovnik(dto::Simulace),
    Geojson(dto::SimulaceGeojson),
}
impl IntoResponse for SimulaceResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Slovnik(data) => Json(data).into_response(),
            Self::Geojson(data) => GeoJson(data).into_response(),
        }
    }
}

/// Validated and typed query parameters, with OpenAPI defaults applied.
/// Unlike plain Axum Query, failures use our documented JSON 422 response.
pub struct ContractQuery<T>(pub T);
impl<T: RequestQuery, S: Send + Sync> FromRequestParts<S> for ContractQuery<T> {
    type Rejection = StubError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // These deprecated parameters have no meaning for the catalog listing.
        // Remove them before validation/deserialization, including repeated or invalid values.
        let uri = if T::OPERATION_ID == "listObory" {
            let pairs: Vec<(String, String)> =
                serde_urlencoded::from_str(parts.uri.query().unwrap_or_default())
                    .map_err(|_| StubError::invalid("query"))?;
            let pairs: Vec<_> = pairs
                .into_iter()
                .filter(|(name, _)| name != "max_min" && name != "scenar")
                .collect();
            let query =
                serde_urlencoded::to_string(pairs).map_err(|_| StubError::invalid("query"))?;
            let mut uri = parts.uri.clone().into_parts();
            uri.path_and_query = Some(
                format!("{}?{query}", parts.uri.path())
                    .parse()
                    .map_err(|_| StubError::invalid("query"))?,
            );
            axum::http::Uri::from_parts(uri).map_err(|_| StubError::invalid("query"))?
        } else {
            parts.uri.clone()
        };
        validate_query(T::OPERATION_ID, uri.query().map(str::to_owned))?;
        Query::<T>::try_from_uri(&uri)
            .map(|Query(query)| Self(query))
            .map_err(|_| StubError::invalid("query"))
    }
}

fn validate_path(operation: &str, field: &str, value: &str) -> Result<(), StubError> {
    let parameter = PARAMETERS[operation]
        .iter()
        .find(|parameter| parameter.location == "path" && parameter.name == field)
        .expect("Path parameter must be declared in openapi.yaml");
    if !parameter.validator.is_valid(&Value::String(value.into())) {
        return Err(StubError::invalid(field));
    }
    Ok(())
}

// Add business logic using `params`, then return the declared response DTO.
pub async fn list_skoly(
    State(pool): State<DbPool>,
    ContractQuery(params): ContractQuery<requests::SkolyQuery>,
) -> Result<GeoJson<dto::SkolyFeatureCollection>, StubError> {
    crate::catalog::schools(pool, params).await.map(GeoJson)
}
pub async fn list_student_skoly(
    State(pool): State<DbPool>,
    Extension(otp): Extension<crate::otp::Client>,
    ContractQuery(params): ContractQuery<requests::StudentSkolyQuery>,
) -> Result<Json<dto::StudentSkoly>, StubError> {
    crate::student::schools(pool, params, otp).await.map(Json)
}
pub async fn get_student_trasa(
    State(pool): State<DbPool>,
    Extension(otp): Extension<crate::otp::Client>,
    ContractQuery(params): ContractQuery<requests::StudentTrasaQuery>,
) -> Result<GeoJson<dto::Trasa>, StubError> {
    crate::student::route(pool, params, otp).await.map(GeoJson)
}
pub async fn list_zsj(
    State(pool): State<DbPool>,
    ContractQuery(_params): ContractQuery<requests::ZsjSeznamQuery>,
) -> Result<Json<Vec<dto::ZsjZaznam>>, StubError> {
    crate::catalog::zsj(pool).await.map(Json)
}
pub async fn get_zsj(
    State(pool): State<DbPool>,
    ContractQuery(params): ContractQuery<requests::ZsjQuery>,
) -> Result<GeoJson<dto::MapaDosahu>, StubError> {
    if params.uroven != requests::Uroven::Zsj {
        return Err(StubError::unimplemented("getZsj"));
    }
    crate::reachability::zsj(pool, params).await.map(GeoJson)
}
pub async fn list_obory(
    State(pool): State<DbPool>,
    ContractQuery(params): ContractQuery<requests::OboryQuery>,
) -> Result<Json<dto::Obory>, StubError> {
    crate::programs::list(pool, params).await.map(Json)
}
pub async fn get_simulace(
    State(pool): State<DbPool>,
    ContractQuery(params): ContractQuery<requests::SimulaceQuery>,
) -> Result<SimulaceResponse, StubError> {
    crate::simulation::get(pool, params).await
}
pub async fn get_skola(
    State(pool): State<DbPool>,
    Path(redizo): Path<requests::Redizo>,
    ContractQuery(params): ContractQuery<requests::SkolaQuery>,
) -> Result<Json<dto::SkolaDetail>, StubError> {
    validate_path("getSkola", "redizo", &redizo.0)?;
    crate::school_detail::get(pool, redizo.0, params)
        .await
        .map(Json)
}

pub async fn get_obor(
    State(pool): State<DbPool>,
    Path(kod): Path<requests::KodOboru>,
    ContractQuery(params): ContractQuery<requests::OborQuery>,
) -> Result<Json<dto::DetailOboru>, StubError> {
    validate_path("getObor", "kod", &kod.0)?;
    crate::program_detail::get(pool, kod.0, params)
        .await
        .map(Json)
}

pub async fn list_obor_zamestnavatele(
    State(pool): State<DbPool>,
    Path(kod): Path<requests::KodOboru>,
    ContractQuery(params): ContractQuery<requests::OborZamestnavateleQuery>,
) -> Result<GeoJson<dto::ZamestnavateleOboru>, StubError> {
    validate_path("listOborZamestnavatele", "kod", &kod.0)?;
    crate::catalog::employers(pool, kod.0, params)
        .await
        .map(GeoJson)
}

/// Build the API with local default OTP settings; tests may inject a client below.
pub fn router(pool: DbPool) -> Router {
    router_with_otp(pool, crate::otp::Client::default())
}

pub fn router_with_otp(pool: DbPool, otp: crate::otp::Client) -> Router {
    let routes = Router::new()
        .route("/skoly", get(list_skoly))
        .route("/skoly/{redizo}", get(get_skola))
        .route("/student/skoly", get(list_student_skoly))
        .route("/student/trasa", get(get_student_trasa))
        .route("/zsj/seznam", get(list_zsj))
        .route("/zsj", get(get_zsj))
        .route("/obory", get(list_obory))
        .route("/obory/{kod}", get(get_obor))
        .route("/obory/{kod}/zamestnavatele", get(list_obor_zamestnavatele))
        .route("/simulace", get(get_simulace))
        .route("/simulace/zmeny", post(post_simulace_zmeny));
    Router::new()
        .nest("/api/v1", routes)
        .route(
            "/openapi.yaml",
            get(|| async { ([(header::CONTENT_TYPE, "application/yaml")], SPEC_YAML) }),
        )
        .route(
            "/docs",
            get(|| async { Html(include_str!("../../swagger.html")) }),
        )
        .layer(Extension(otp))
        .layer(axum::middleware::from_fn(crate::logging::log_request))
        .with_state(pool)
}

impl From<diesel::result::Error> for StubError {
    fn from(error: diesel::result::Error) -> Self {
        crate::catalog::database_error(error)
    }
}

pub async fn post_simulace_zmeny(
    State(pool): State<DbPool>,
    body: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<crate::batch_simulation::BatchResponse, StubError> {
    static VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        schema_validator(
            &SPEC["paths"]["/simulace/zmeny"]["post"]["requestBody"]["content"]["application/json"]
                ["schema"],
        )
    });
    let Json(value) = body.map_err(|_| StubError::invalid("body"))?;
    if !VALIDATOR.is_valid(&value) {
        return Err(StubError::invalid("body"));
    }
    let request: requests::BatchSimulaceRequest =
        serde_json::from_value(value).map_err(|_| StubError::invalid("body"))?;
    crate::batch_simulation::run(pool, request).await
}
