//! Generic TOON value type for dynamic content.
//!
//! The [`Value`] enum represents any valid TOON value, similar to `serde_json::Value`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// An order-preserving map type used for TOON objects.
///
/// Uses [`IndexMap`] to preserve the insertion order of keys, which is important
/// for maintaining consistent serialization output.
pub type Map<K, V> = IndexMap<K, V>;

/// Represents any valid TOON value.
///
/// This type is useful when you need to work with TOON data dynamically without
/// knowing the schema ahead of time.
///
/// # Examples
///
/// ```
/// use serde_toon2::{Value, Map, Number};
///
/// // Create a simple value
/// let name = Value::String("Ada".to_string());
///
/// // Create an object
/// let mut map = Map::new();
/// map.insert("name".to_string(), Value::String("Ada".to_string()));
/// map.insert("age".to_string(), Value::Number(Number::U64(42)));
/// let obj = Value::Object(map);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Represents a null value.
    Null,
    /// Represents a boolean value.
    Bool(bool),
    /// Represents a numeric value.
    Number(Number),
    /// Represents a string value.
    String(String),
    /// Represents an array of values.
    Array(Vec<Value>),
    /// Represents an object (map of string keys to values).
    Object(Map<String, Value>),
}

/// Represents a TOON number.
///
/// TOON supports signed integers, unsigned integers, and floating point numbers.
///
/// # Examples
///
/// ```
/// use serde_toon2::{Number, Value};
///
/// let int = Number::I64(-42);
/// let uint = Number::U64(42);
/// let float = Number::F64(3.14);
///
/// assert_eq!(int.as_i64(), Some(-42));
/// assert_eq!(uint.as_u64(), Some(42));
/// assert_eq!(float.as_f64(), 3.14);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// A signed 64-bit integer.
    I64(i64),
    /// An unsigned 64-bit integer.
    U64(u64),
    /// A 64-bit floating point number.
    F64(f64),
}

impl Number {
    /// Tries to convert this number to an `i64`.
    ///
    /// Returns `None` if the conversion would lose precision or overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Number;
    ///
    /// assert_eq!(Number::I64(42).as_i64(), Some(42));
    /// assert_eq!(Number::U64(42).as_i64(), Some(42));
    /// assert_eq!(Number::F64(42.0).as_i64(), Some(42));
    /// assert_eq!(Number::F64(42.5).as_i64(), None);
    /// ```
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

    /// Tries to convert this number to a `u64`.
    ///
    /// Returns `None` if the conversion would lose precision or overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Number;
    ///
    /// assert_eq!(Number::U64(42).as_u64(), Some(42));
    /// assert_eq!(Number::I64(42).as_u64(), Some(42));
    /// assert_eq!(Number::I64(-42).as_u64(), None);
    /// assert_eq!(Number::F64(42.0).as_u64(), Some(42));
    /// assert_eq!(Number::F64(42.5).as_u64(), None);
    /// ```
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

    /// Converts this number to an `f64`.
    ///
    /// This conversion is always possible, though very large integers may lose precision.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Number;
    ///
    /// assert_eq!(Number::I64(42).as_f64(), 42.0);
    /// assert_eq!(Number::U64(42).as_f64(), 42.0);
    /// assert_eq!(Number::F64(3.14).as_f64(), 3.14);
    /// ```
    pub fn as_f64(&self) -> f64 {
        match self {
            Number::I64(n) => *n as f64,
            Number::U64(n) => *n as f64,
            Number::F64(n) => *n,
        }
    }

    /// Returns `true` if this number is stored as an `i64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Number;
    ///
    /// assert!(Number::I64(42).is_i64());
    /// assert!(!Number::U64(42).is_i64());
    /// assert!(!Number::F64(42.0).is_i64());
    /// ```
    pub fn is_i64(&self) -> bool {
        matches!(self, Number::I64(_))
    }

    /// Returns `true` if this number is stored as a `u64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Number;
    ///
    /// assert!(Number::U64(42).is_u64());
    /// assert!(!Number::I64(42).is_u64());
    /// assert!(!Number::F64(42.0).is_u64());
    /// ```
    pub fn is_u64(&self) -> bool {
        matches!(self, Number::U64(_))
    }

    /// Returns `true` if this number is stored as an `f64`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Number;
    ///
    /// assert!(Number::F64(3.14).is_f64());
    /// assert!(!Number::I64(42).is_f64());
    /// assert!(!Number::U64(42).is_f64());
    /// ```
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
    /// Returns `true` if this value is null.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// assert!(Value::Null.is_null());
    /// assert!(!Value::Bool(false).is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// If this value is a boolean, returns it. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// assert_eq!(Value::Bool(true).as_bool(), Some(true));
    /// assert_eq!(Value::Null.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// If this value is a number, tries to convert it to `i64`. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// let v: Value = 42i64.into();
    /// assert_eq!(v.as_i64(), Some(42));
    ///
    /// let v = Value::String("42".to_string());
    /// assert_eq!(v.as_i64(), None);
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// If this value is a number, tries to convert it to `u64`. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// let v: Value = 42u64.into();
    /// assert_eq!(v.as_u64(), Some(42));
    ///
    /// let v: Value = (-42i64).into();
    /// assert_eq!(v.as_u64(), None);
    /// ```
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) => n.as_u64(),
            _ => None,
        }
    }

    /// If this value is a number, converts it to `f64`. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// let v: Value = 3.14.into();
    /// assert_eq!(v.as_f64(), Some(3.14));
    ///
    /// let v = Value::String("3.14".to_string());
    /// assert_eq!(v.as_f64(), None);
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(n.as_f64()),
            _ => None,
        }
    }

    /// If this value is a string, returns a reference to it. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// let v = Value::String("hello".to_string());
    /// assert_eq!(v.as_str(), Some("hello"));
    ///
    /// let v = Value::Null;
    /// assert_eq!(v.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// If this value is an array, returns a reference to it. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Value;
    ///
    /// let v = Value::Array(vec![Value::String("a".to_string())]);
    /// assert!(v.as_array().is_some());
    ///
    /// let v = Value::Null;
    /// assert!(v.as_array().is_none());
    /// ```
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// If this value is an object, returns a reference to it. Otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::{Value, Map};
    ///
    /// let mut map = Map::new();
    /// map.insert("key".to_string(), Value::String("value".to_string()));
    /// let v = Value::Object(map);
    /// assert!(v.as_object().is_some());
    ///
    /// let v = Value::Null;
    /// assert!(v.as_object().is_none());
    /// ```
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
