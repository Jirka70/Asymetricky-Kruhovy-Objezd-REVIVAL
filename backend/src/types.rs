//! Domain values shared by query and response models.
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

/// NSP suitability: numeric values on the wire, descriptive values in Rust.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Vhodnost {
    Nejvhodnejsi = 1,
    #[default]
    Vhodne = 2,
}
impl Serialize for Vhodnost {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*self as u8)
    }
}
impl<'de> Deserialize<'de> for Vhodnost {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u8::deserialize(deserializer)? {
            1 => Ok(Self::Nejvhodnejsi),
            2 => Ok(Self::Vhodne),
            _ => Err(serde::de::Error::custom("vhodnost must be 1 or 2")),
        }
    }
}
impl JsonSchema for Vhodnost {
    fn schema_name() -> Cow<'static, str> {
        "Vhodnost".into()
    }
    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"integer", "enum":[1, 2]})
    }
}
