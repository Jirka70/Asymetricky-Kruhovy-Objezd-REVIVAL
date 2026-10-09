//! OpenAPI handlers, query validation, and stubs for analytical operations.
use crate::{
    db::DbPool,
    dto, models,
    requests::{self, RequestQuery},
    schema,
};
use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, Query, State},
    http::{StatusCode, header, request::Parts},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use bigdecimal::ToPrimitive;
use diesel::prelude::*;
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
}
pub const OPERATIONS: &[Operation] = &[
    Operation {
        path: "/skoly",
        id: "listSkoly",
    },
    Operation {
        path: "/skoly/{redizo}",
        id: "getSkola",
    },
    Operation {
        path: "/student/skoly",
        id: "listStudentSkoly",
    },
    Operation {
        path: "/student/trasa",
        id: "getStudentTrasa",
    },
    Operation {
        path: "/zsj",
        id: "getZsj",
    },
    Operation {
        path: "/obory",
        id: "listObory",
    },
    Operation {
        path: "/obory/{kod}",
        id: "getObor",
    },
    Operation {
        path: "/obory/{kod}/zamestnavatele",
        id: "listOborZamestnavatele",
    },
    Operation {
        path: "/simulace",
        id: "getSimulace",
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
            let parameters = SPEC["paths"][operation.path]["get"]["parameters"]
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
    fn invalid(field: &str) -> Self {
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

fn internal_error() -> StubError {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "interni_chyba",
        "Nepodařilo se načíst detail školy.",
    )
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
        validate_query(T::OPERATION_ID, parts.uri.query().map(str::to_owned))?;
        Query::<T>::try_from_uri(&parts.uri)
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
    ContractQuery(_params): ContractQuery<requests::StudentSkolyQuery>,
) -> Result<Json<dto::StudentSkoly>, StubError> {
    Err(StubError::unimplemented("listStudentSkoly"))
}
pub async fn get_student_trasa(
    ContractQuery(_params): ContractQuery<requests::StudentTrasaQuery>,
) -> Result<GeoJson<dto::Trasa>, StubError> {
    Err(StubError::unimplemented("getStudentTrasa"))
}
pub async fn get_zsj(
    ContractQuery(_params): ContractQuery<requests::ZsjQuery>,
) -> Result<GeoJson<dto::MapaDosahu>, StubError> {
    Err(StubError::unimplemented("getZsj"))
}
pub async fn list_obory(
    ContractQuery(_params): ContractQuery<requests::OboryQuery>,
) -> Result<Json<dto::Obory>, StubError> {
    Err(StubError::unimplemented("listObory"))
}
pub async fn get_simulace(
    ContractQuery(_params): ContractQuery<requests::SimulaceQuery>,
) -> Result<SimulaceResponse, StubError> {
    Err(StubError::unimplemented("getSimulace"))
}
pub async fn get_skola(
    State(pool): State<DbPool>,
    Path(redizo): Path<requests::Redizo>,
    ContractQuery(_params): ContractQuery<requests::SkolaQuery>,
) -> Result<Json<dto::SkolaDetail>, StubError> {
    validate_path("getSkola", "redizo", &redizo.0)?;

    let detail = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|error| {
            tracing::error!(%error, "Cannot obtain database connection");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "databaze_nedostupna",
                "Databáze je dočasně nedostupná.",
            )
        })?;

        let school = schema::stredni_skoly::table
            .find(&redizo.0)
            .select(models::Skola::as_select())
            .first::<models::Skola>(&mut conn)
            .optional()
            .map_err(|error| {
                tracing::error!(%error, "Cannot load school");
                internal_error()
            })?
            .ok_or_else(|| {
                api_error(
                    StatusCode::NOT_FOUND,
                    "skola_nenalezena",
                    "Škola nebyla nalezena.",
                )
            })?;

        let offers = schema::nabidka_oboru::table
            .inner_join(schema::obory::table)
            .filter(schema::nabidka_oboru::redizo.eq(&redizo.0))
            .select((models::NabidkaOboru::as_select(), models::Obor::as_select()))
            .load::<(models::NabidkaOboru, models::Obor)>(&mut conn)
            .map_err(|error| {
                tracing::error!(%error, "Cannot load school offerings");
                internal_error()
            })?;

        let nabidky = offers
            .into_iter()
            .map(|(offer, obor)| {
                let forma = match offer.forma_studia.as_str() {
                    "den" => dto::NabidkaForma::Den,
                    "dal" => dto::NabidkaForma::Dal,
                    _ => return Err(internal_error()),
                };

                Ok(dto::Nabidka {
                    kod_oboru: obor.kod,
                    nazev_oboru: obor.nazev,
                    zamereni: offer.display_name,
                    stupen: None,
                    forma,
                    delka_let: Some(i64::from(offer.delka_studia)),
                    kapacita: i64::from(offer.pocet_prijimanych),
                    prihlasky: i64::from(offer.loni_pocet_prihlasek),
                    prihlasky_na_misto: (offer.pocet_prijimanych > 0).then(|| {
                        f64::from(offer.loni_pocet_prihlasek) / f64::from(offer.pocet_prijimanych)
                    }),
                    index_pretlaku: None,
                })
            })
            .collect::<Result<Vec<_>, StubError>>()?;

        // These fields are nullable in the DB but required by the API.
        let nazev = school.nazev.ok_or_else(internal_error)?;
        let lat = school
            .lat
            .and_then(|value| value.to_f64())
            .filter(|value| value.is_finite())
            .ok_or_else(internal_error)?;
        let lon = school
            .lon
            .and_then(|value| value.to_f64())
            .filter(|value| value.is_finite())
            .ok_or_else(internal_error)?;

        Ok::<_, StubError>(dto::SkolaDetail {
            redizo: school.redizo,
            nazev,
            adresa: school.adresa,
            web: school.web,
            lat,
            lon,
            nabidky,
            spadovost: dto::SkolaDetailSpadovost {
                deti_v_dosahu: None,
                obce: None,
            },
            meta: None,
        })
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "School query task failed");
        internal_error()
    })??;

    Ok(Json(detail))
}
pub async fn get_obor(
    Path(kod): Path<requests::KodOboru>,
    ContractQuery(_params): ContractQuery<requests::OborQuery>,
) -> Result<Json<dto::DetailOboru>, StubError> {
    validate_path("getObor", "kod", &kod.0)?;
    Err(StubError::unimplemented("getObor"))
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

/// Database reads use the supplied pool; analytical operations remain stubs.
pub fn router(pool: DbPool) -> Router {
    let routes = Router::new()
        .route("/skoly", get(list_skoly))
        .route("/skoly/{redizo}", get(get_skola))
        .route("/student/skoly", get(list_student_skoly))
        .route("/student/trasa", get(get_student_trasa))
        .route("/zsj", get(get_zsj))
        .route("/obory", get(list_obory))
        .route("/obory/{kod}", get(get_obor))
        .route("/obory/{kod}/zamestnavatele", get(list_obor_zamestnavatele))
        .route("/simulace", get(get_simulace));
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
        .with_state(pool)
}
