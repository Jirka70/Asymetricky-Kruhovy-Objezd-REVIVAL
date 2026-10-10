//! Typed query parameters for the public OpenAPI endpoints.
use crate::dto::Stupen;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::borrow::Cow;

/// Identifies the OpenAPI operation whose query schemas must be validated.
pub trait RequestQuery: DeserializeOwned {
    const OPERATION_ID: &'static str;
}

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub String);
    };
}
identifier!(Redizo);
identifier!(KodOboru);
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Scenar {
    #[default]
    Rano,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Uroven {
    Orp,
    #[default]
    Obec,
    Zsj,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Forma {
    #[default]
    Den,
    Dal,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Slovnik,
    Geojson,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Razeni {
    #[serde(rename = "index_pretlaku")]
    IndexPretlaku,
    #[serde(rename = "-index_pretlaku")]
    IndexPretlakuSestupne,
    #[serde(rename = "volna_mista_na_misto")]
    VolnaMistaNaMisto,
    #[serde(rename = "-volna_mista_na_misto")]
    VolnaMistaNaMistoSestupne,
    #[default]
    #[serde(rename = "nazev")]
    Nazev,
}

/// The HTTP representation is `H,M`; business logic receives education enums.
#[derive(Debug, Clone, PartialEq)]
pub struct Stupne(pub Vec<Stupen>);
impl<'de> Deserialize<'de> for Stupne {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.split(',')
            .map(|value| {
                serde_json::from_value(serde_json::Value::String(value.into()))
                    .map_err(serde::de::Error::custom)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}
impl Serialize for Stupne {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let values: Vec<_> = self
            .0
            .iter()
            .map(|value| match value {
                Stupen::C => "C",
                Stupen::E => "E",
                Stupen::H => "H",
                Stupen::K => "K",
                Stupen::L => "L",
                Stupen::M => "M",
            })
            .collect();
        serializer.serialize_str(&values.join(","))
    }
}
impl JsonSchema for Stupne {
    fn schema_name() -> Cow<'static, str> {
        "Stupne".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }
}

/// OpenAPI declares an unrestricted CSV string; known-signal lookup belongs to business logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalFilter(pub Vec<String>);
impl<'de> Deserialize<'de> for SignalFilter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)
            .map(|raw| Self(raw.split(',').map(str::to_owned).collect()))
    }
}
impl Serialize for SignalFilter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.join(","))
    }
}
impl JsonSchema for SignalFilter {
    fn schema_name() -> Cow<'static, str> {
        "SignalFilter".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        String::json_schema(generator)
    }
}

fn default_max_min() -> u16 {
    120
}
fn default_kandidatu() -> u8 {
    5
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkolyQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obor: Option<KodOboru>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stupen: Option<Stupne>,
    #[serde(default)]
    pub forma: Forma,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkolaQuery {
    #[serde(default = "default_max_min")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StudentSkolyQuery {
    pub lat: f64,
    pub lon: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obor: Option<KodOboru>,
    #[serde(default)]
    pub forma: Forma,
    #[serde(default = "default_max_min")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StudentTrasaQuery {
    pub lat: f64,
    pub lon: f64,
    pub redizo: Redizo,
    #[serde(default)]
    pub scenar: Scenar,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ZsjSeznamQuery {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ZsjQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obor: Option<KodOboru>,
    #[serde(default)]
    pub forma: Forma,
    #[serde(default)]
    pub uroven: Uroven,
    #[serde(default = "default_max_min")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OboryQuery {
    #[serde(default)]
    pub forma: Forma,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stupen: Option<Stupne>,
    #[serde(default = "default_max_min")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<SignalFilter>,
    #[serde(default)]
    pub razeni: Razeni,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OborQuery {
    #[serde(default = "default_max_min")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
    #[serde(default = "default_kandidatu")]
    pub kandidatu: u8,
}

fn default_jen_ss() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OborZamestnavateleQuery {
    #[serde(default)]
    pub vhodnost: crate::types::Vhodnost,
    #[serde(default = "default_jen_ss")]
    pub jen_ss: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimulaceQuery {
    pub redizo: Redizo,
    pub obor: KodOboru,
    pub kapacita: i32,
    #[serde(default)]
    pub uroven: Uroven,
    #[serde(default = "default_max_min")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
    #[serde(default)]
    pub format: Format,
}

macro_rules! operation {
    ($query:ty, $id:literal) => {
        impl RequestQuery for $query {
            const OPERATION_ID: &'static str = $id;
        }
    };
}
operation!(SkolyQuery, "listSkoly");
operation!(SkolaQuery, "getSkola");
operation!(StudentSkolyQuery, "listStudentSkoly");
operation!(StudentTrasaQuery, "getStudentTrasa");
operation!(ZsjSeznamQuery, "listZsj");
operation!(ZsjQuery, "getZsj");
operation!(OboryQuery, "listObory");
operation!(OborQuery, "getObor");
operation!(OborZamestnavateleQuery, "listOborZamestnavatele");
operation!(SimulaceQuery, "getSimulace");

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchSimulaceRequest {
    pub obor: KodOboru,
    pub zmeny: Vec<ZmenaKapacity>,
    #[serde(default = "default_max_min", deserialize_with = "json_integer")]
    pub max_min: u16,
    #[serde(default)]
    pub scenar: Scenar,
    #[serde(default)]
    pub uroven: Uroven,
    #[serde(default)]
    pub format: Format,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ZmenaKapacity {
    pub redizo: Redizo,
    #[serde(deserialize_with = "json_integer")]
    pub zmena_kapacity: i64,
}

// JSON Schema integers include numbers with no fractional component (e.g. 30.0).
fn json_integer<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<i64>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let integer = value
        .as_i64()
        .or_else(|| {
            value
                .as_f64()
                .filter(|number| {
                    number.fract() == 0.0
                        && *number >= i64::MIN as f64
                        && *number < -(i64::MIN as f64)
                })
                .map(|number| number as i64)
        })
        .ok_or_else(|| serde::de::Error::custom("Expected an integer-valued JSON number"))?;
    T::try_from(integer)
        .map_err(|_| serde::de::Error::custom("Integer is outside the supported range"))
}
