//! Reject duplicate object members before serde_json::Value can discard them.

use std::fmt;

use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueVisitor).map(Self)
    }
}

struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value with unique object keys")
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(value.into())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(UniqueValue(value)) = seq.next_element()? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(A::Error::custom(format!("duplicate JSON member '{key}'")));
            }
            let UniqueValue(value) = map.next_value()?;
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}

pub(super) fn parse(raw: &[u8]) -> serde_json::Result<Vec<Value>> {
    let entries = serde_json::from_slice::<Vec<UniqueValue>>(raw)?;
    Ok(entries.into_iter().map(|entry| entry.0).collect())
}
