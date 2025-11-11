use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

pub type Map<K, V> = IndexMap<K, V>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    I64(i64),
    U64(u64),
    F64(f64),
}

impl Number {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Number::I64(n) => Some(*n),
            Number::U64(n) => i64::try_from(*n).ok(),
            Number::F64(n) => {
                if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                    Some(*n as i64)
                } else {
                    None
                }
            }
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Number::I64(n) => u64::try_from(*n).ok(),
            Number::U64(n) => Some(*n),
            Number::F64(n) => {
                if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 {
                    Some(*n as u64)
                } else {
                    None
                }
            }
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            Number::I64(n) => *n as f64,
            Number::U64(n) => *n as f64,
            Number::F64(n) => *n,
        }
    }

    pub fn is_i64(&self) -> bool {
        matches!(self, Number::I64(_))
    }

    pub fn is_u64(&self) -> bool {
        matches!(self, Number::U64(_))
    }

    pub fn is_f64(&self) -> bool {
        matches!(self, Number::F64(_))
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Number::I64(n) => serializer.serialize_i64(*n),
            Number::U64(n) => serializer.serialize_u64(*n),
            Number::F64(n) => serializer.serialize_f64(*n),
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NumberVisitor;

        impl<'de> serde::de::Visitor<'de> for NumberVisitor {
            type Value = Number;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a number")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(Number::I64(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(Number::U64(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(Number::F64(value))
            }
        }

        deserializer.deserialize_any(NumberVisitor)
    }
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n.as_f64()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Map<String, Value>> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(Number::I64(n))
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Value::Number(Number::U64(n))
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(Number::F64(n))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<Map<String, Value>> for Value {
    fn from(m: Map<String, Value>) -> Self {
        Value::Object(m)
    }
}
