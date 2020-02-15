use serde::de::{Deserialize, Deserializer, Error, Visitor};
use std::convert::TryInto;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum FieldValue {
    String(String),
    Float(f64),
    Integer(i64),
    Boolean(bool),
}

struct FieldValueVisitor;

impl<'de> Visitor<'de> for FieldValueVisitor {
    type Value = FieldValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a boolean, a float, an integer or a string")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FieldValue::Boolean(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FieldValue::Integer(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FieldValue::Integer(v.try_into().map_err(Error::custom)?))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FieldValue::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FieldValue::String(v.to_owned()))
    }
}

impl<'de> Deserialize<'de> for FieldValue {
    fn deserialize<D>(deserializer: D) -> Result<FieldValue, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FieldValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    #[allow(clippy::approx_constant)]
    fn decode_test() {
        let encoded = "[true, 3.14, 42, -42, \"hello world\"]";
        let values: Vec<FieldValue> = from_str(encoded).unwrap();
        assert_eq!(5, values.len());
        assert_eq!(FieldValue::Boolean(true), values[0]);
        assert_eq!(FieldValue::Float(3.14), values[1]);
        assert_eq!(FieldValue::Integer(42), values[2]);
        assert_eq!(FieldValue::Integer(-42), values[3]);
        assert_eq!(FieldValue::String("hello world".to_owned()), values[4]);
    }
}
