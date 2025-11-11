use crate::error::{Error, Result};
use crate::options::{Delimiter, EncoderOptions};
use crate::value::{Map, Number, Value};
use serde::ser;
use std::io::Write;

pub struct Serializer<W> {
    writer: W,
    options: EncoderOptions,
    depth: usize,
    document_delimiter: Delimiter,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W, options: EncoderOptions) -> Self {
        let document_delimiter = options.delimiter;
        Serializer {
            writer,
            options,
            depth: 0,
            document_delimiter,
        }
    }

    fn indent(&self) -> String {
        " ".repeat(self.depth * self.options.indent)
    }

    fn needs_quoting(&self, s: &str, active_delimiter: Delimiter) -> bool {
        if s.is_empty() {
            return true;
        }

        if s.starts_with(|c: char| c.is_whitespace()) || s.ends_with(|c: char| c.is_whitespace())
        {
            return true;
        }

        if s == "true" || s == "false" || s == "null" {
            return true;
        }

        if s.starts_with('-') {
            return true;
        }

        if s.contains(active_delimiter.as_char())
            || s.contains(':')
            || s.contains('"')
            || s.contains('\\')
            || s.contains('[')
            || s.contains(']')
            || s.contains('{')
            || s.contains('}')
            || s.contains('\n')
            || s.contains('\r')
            || s.contains('\t')
        {
            return true;
        }

        let numeric_pattern = regex::Regex::new(r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$").unwrap();
        if numeric_pattern.is_match(s) {
            return true;
        }

        let leading_zero_pattern = regex::Regex::new(r"^0\d+$").unwrap();
        if leading_zero_pattern.is_match(s) {
            return true;
        }

        false
    }

    fn escape_string(&self, s: &str) -> String {
        let mut result = String::new();
        for ch in s.chars() {
            match ch {
                '\\' => result.push_str("\\\\"),
                '"' => result.push_str("\\\""),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                _ => result.push(ch),
            }
        }
        result
    }

    fn write_string(&mut self, s: &str, active_delimiter: Delimiter) -> Result<()> {
        if self.needs_quoting(s, active_delimiter) {
            write!(self.writer, "\"{}\"", self.escape_string(s))?;
        } else {
            write!(self.writer, "{}", s)?;
        }
        Ok(())
    }

    fn format_number(&self, num: &Number) -> String {
        match num {
            Number::I64(n) => n.to_string(),
            Number::U64(n) => n.to_string(),
            Number::F64(f) => {
                if f.is_nan() || f.is_infinite() {
                    return "null".to_string();
                }
                if *f == -0.0 {
                    return "0".to_string();
                }
                if f.fract() == 0.0 {
                    format!("{:.0}", f)
                } else {
                    let mut s = f.to_string();
                    if s.contains('e') || s.contains('E') {
                        let val = *f;
                        s = format!("{:.17}", val);
                        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
                    }
                    s
                }
            }
        }
    }

    pub fn serialize_value(&mut self, value: &Value) -> Result<()> {
        self.serialize_value_with_key(value, None, self.document_delimiter)
    }

    fn serialize_value_with_key(
        &mut self,
        value: &Value,
        key: Option<&str>,
        active_delimiter: Delimiter,
    ) -> Result<()> {
        match value {
            Value::Null => {
                if let Some(k) = key {
                    write!(self.writer, "{}{}: null", self.indent(), k)?;
                } else {
                    write!(self.writer, "null")?;
                }
            }
            Value::Bool(b) => {
                if let Some(k) = key {
                    write!(self.writer, "{}{}: {}", self.indent(), k, b)?;
                } else {
                    write!(self.writer, "{}", b)?;
                }
            }
            Value::Number(n) => {
                let formatted = self.format_number(n);
                if let Some(k) = key {
                    write!(self.writer, "{}{}: {}", self.indent(), k, formatted)?;
                } else {
                    write!(self.writer, "{}", formatted)?;
                }
            }
            Value::String(s) => {
                if let Some(k) = key {
                    write!(self.writer, "{}{}: ", self.indent(), k)?;
                    self.write_string(s, active_delimiter)?;
                } else {
                    self.write_string(s, active_delimiter)?;
                }
            }
            Value::Array(arr) => {
                self.serialize_array(arr, key, active_delimiter)?;
            }
            Value::Object(obj) => {
                self.serialize_object(obj, key)?;
            }
        }
        Ok(())
    }

    fn serialize_array(
        &mut self,
        arr: &[Value],
        key: Option<&str>,
        parent_delimiter: Delimiter,
    ) -> Result<()> {
        let len = arr.len();
        let active_delimiter = parent_delimiter;

        if arr.is_empty() {
            let header_delim = active_delimiter.header_marker();
            if let Some(k) = key {
                write!(self.writer, "{}{}[{}{}]:", self.indent(), k, len, header_delim)?;
            } else {
                write!(self.writer, "[{}{}]:", len, header_delim)?;
            }
            return Ok(());
        }

        if self.is_primitive_array(arr) {
            self.serialize_primitive_array(arr, key, active_delimiter)?;
        } else if self.is_array_of_arrays(arr) {
            self.serialize_array_of_arrays(arr, key, active_delimiter)?;
        } else if let Some((is_tabular, fields)) = self.detect_tabular(arr) {
            if is_tabular {
                self.serialize_tabular_array(arr, key, &fields, active_delimiter)?;
            } else {
                self.serialize_mixed_array(arr, key, active_delimiter)?;
            }
        } else {
            self.serialize_mixed_array(arr, key, active_delimiter)?;
        }

        Ok(())
    }

    fn is_primitive_array(&self, arr: &[Value]) -> bool {
        arr.iter().all(|v| matches!(v, Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)))
    }

    fn is_array_of_arrays(&self, arr: &[Value]) -> bool {
        if arr.is_empty() {
            return false;
        }
        arr.iter().all(|v| matches!(v, Value::Array(inner) if self.is_primitive_array(inner)))
    }

    fn detect_tabular(&self, arr: &[Value]) -> Option<(bool, Vec<String>)> {
        if arr.is_empty() {
            return Some((false, vec![]));
        }

        let mut all_objects = true;
        let mut fields: Option<Vec<String>> = None;

        for item in arr {
            match item {
                Value::Object(obj) => {
                    let item_keys: Vec<String> = obj.keys().cloned().collect();

                    for val in obj.values() {
                        if !matches!(val, Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) {
                            return Some((false, vec![]));
                        }
                    }

                    if let Some(ref expected_fields) = fields {
                        let keys_set: std::collections::HashSet<_> = item_keys.iter().collect();
                        let expected_set: std::collections::HashSet<_> = expected_fields.iter().collect();
                        if keys_set != expected_set {
                            return Some((false, vec![]));
                        }
                    } else {
                        fields = Some(item_keys);
                    }
                }
                _ => {
                    all_objects = false;
                    break;
                }
            }
        }

        if all_objects && fields.is_some() {
            Some((true, fields.unwrap()))
        } else {
            Some((false, vec![]))
        }
    }

    fn serialize_primitive_array(
        &mut self,
        arr: &[Value],
        key: Option<&str>,
        active_delimiter: Delimiter,
    ) -> Result<()> {
        let len = arr.len();
        let header_delim = active_delimiter.header_marker();

        if let Some(k) = key {
            write!(self.writer, "{}{}[{}{}]: ", self.indent(), k, len, header_delim)?;
        } else {
            write!(self.writer, "[{}{}]: ", len, header_delim)?;
        }

        for (i, val) in arr.iter().enumerate() {
            if i > 0 {
                write!(self.writer, "{}", active_delimiter.as_str())?;
            }
            match val {
                Value::Null => write!(self.writer, "null")?,
                Value::Bool(b) => write!(self.writer, "{}", b)?,
                Value::Number(n) => write!(self.writer, "{}", self.format_number(n))?,
                Value::String(s) => self.write_string(s, active_delimiter)?,
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    fn serialize_array_of_arrays(
        &mut self,
        arr: &[Value],
        key: Option<&str>,
        active_delimiter: Delimiter,
    ) -> Result<()> {
        let len = arr.len();
        let header_delim = active_delimiter.header_marker();

        if let Some(k) = key {
            write!(self.writer, "{}{}[{}{}]:", self.indent(), k, len, header_delim)?;
        } else {
            write!(self.writer, "[{}{}]:", len, header_delim)?;
        }

        self.depth += 1;
        for inner_arr in arr {
            if let Value::Array(inner) = inner_arr {
                write!(self.writer, "\n{}- ", self.indent())?;
                self.serialize_primitive_array(inner, None, active_delimiter)?;
            }
        }
        self.depth -= 1;

        Ok(())
    }

    fn serialize_tabular_array(
        &mut self,
        arr: &[Value],
        key: Option<&str>,
        fields: &[String],
        active_delimiter: Delimiter,
    ) -> Result<()> {
        let len = arr.len();
        let header_delim = active_delimiter.header_marker();

        if let Some(k) = key {
            write!(self.writer, "{}{}[{}{}]{{", self.indent(), k, len, header_delim)?;
        } else {
            write!(self.writer, "[{}{}]{{", len, header_delim)?;
        }

        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                write!(self.writer, "{}", active_delimiter.as_str())?;
            }
            if self.key_needs_quoting(field) {
                write!(self.writer, "\"{}\"", self.escape_string(field))?;
            } else {
                write!(self.writer, "{}", field)?;
            }
        }
        write!(self.writer, "}}:")?;

        self.depth += 1;
        for obj in arr {
            if let Value::Object(map) = obj {
                write!(self.writer, "\n{}", self.indent())?;
                for (i, field_name) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(self.writer, "{}", active_delimiter.as_str())?;
                    }
                    if let Some(val) = map.get(field_name) {
                        match val {
                            Value::Null => write!(self.writer, "null")?,
                            Value::Bool(b) => write!(self.writer, "{}", b)?,
                            Value::Number(n) => write!(self.writer, "{}", self.format_number(n))?,
                            Value::String(s) => self.write_string(s, active_delimiter)?,
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
        self.depth -= 1;

        Ok(())
    }

    fn serialize_mixed_array(
        &mut self,
        arr: &[Value],
        key: Option<&str>,
        active_delimiter: Delimiter,
    ) -> Result<()> {
        let len = arr.len();
        let header_delim = active_delimiter.header_marker();

        if let Some(k) = key {
            write!(self.writer, "{}{}[{}{}]:", self.indent(), k, len, header_delim)?;
        } else {
            write!(self.writer, "[{}{}]:", len, header_delim)?;
        }

        self.depth += 1;
        for item in arr {
            write!(self.writer, "\n{}- ", self.indent())?;
            match item {
                Value::Null => write!(self.writer, "null")?,
                Value::Bool(b) => write!(self.writer, "{}", b)?,
                Value::Number(n) => write!(self.writer, "{}", self.format_number(n))?,
                Value::String(s) => {
                    self.write_string(s, active_delimiter)?;
                }
                Value::Array(inner) => {
                    self.serialize_primitive_array(inner, None, active_delimiter)?;
                }
                Value::Object(obj) => {
                    self.serialize_object_as_list_item(obj, active_delimiter)?;
                }
            }
        }
        self.depth -= 1;

        Ok(())
    }

    fn serialize_object_as_list_item(
        &mut self,
        obj: &Map<String, Value>,
        active_delimiter: Delimiter,
    ) -> Result<()> {
        if obj.is_empty() {
            return Ok(());
        }

        let mut first = true;
        for (key, value) in obj {
            if first {
                let quoted_key = if self.key_needs_quoting(key) {
                    format!("\"{}\"", self.escape_string(key))
                } else {
                    key.clone()
                };

                match value {
                    Value::Null => write!(self.writer, "{}: null", quoted_key)?,
                    Value::Bool(b) => write!(self.writer, "{}: {}", quoted_key, b)?,
                    Value::Number(n) => write!(self.writer, "{}: {}", quoted_key, self.format_number(n))?,
                    Value::String(s) => {
                        write!(self.writer, "{}: ", quoted_key)?;
                        self.write_string(s, self.document_delimiter)?;
                    }
                    Value::Array(arr) => {
                        self.serialize_array(arr, Some(key), active_delimiter)?;
                    }
                    Value::Object(nested) => {
                        write!(self.writer, "{}:", quoted_key)?;
                        self.depth += 1;
                        for (nested_key, nested_val) in nested {
                            write!(self.writer, "\n")?;
                            self.serialize_value_with_key(nested_val, Some(nested_key), self.document_delimiter)?;
                        }
                        self.depth -= 1;
                    }
                }
                first = false;
            } else {
                write!(self.writer, "\n")?;
                self.serialize_value_with_key(value, Some(key), self.document_delimiter)?;
            }
        }

        Ok(())
    }

    fn key_needs_quoting(&self, key: &str) -> bool {
        if key.is_empty() {
            return true;
        }

        let valid_unquoted = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_.]*$").unwrap();
        if !valid_unquoted.is_match(key) {
            return true;
        }

        if key.contains('\n') || key.contains('\r') || key.contains('\t') || key.contains('\\') || key.contains('"') {
            return true;
        }

        false
    }

    fn serialize_object(&mut self, obj: &Map<String, Value>, key: Option<&str>) -> Result<()> {
        if obj.is_empty() {
            if let Some(k) = key {
                write!(self.writer, "{}{}:", self.indent(), k)?;
            }
            return Ok(());
        }

        if let Some(k) = key {
            write!(self.writer, "{}{}:", self.indent(), k)?;
            self.depth += 1;
            for (obj_key, obj_val) in obj {
                write!(self.writer, "\n")?;
                self.serialize_value_with_key(obj_val, Some(obj_key), self.document_delimiter)?;
            }
            self.depth -= 1;
        } else {
            for (i, (obj_key, obj_val)) in obj.iter().enumerate() {
                if i > 0 {
                    write!(self.writer, "\n")?;
                }
                self.serialize_value_with_key(obj_val, Some(obj_key), self.document_delimiter)?;
            }
        }

        Ok(())
    }
}

pub fn to_string<T: ser::Serialize>(value: &T) -> Result<String> {
    to_string_with_options(value, EncoderOptions::default())
}

pub fn to_string_with_options<T: ser::Serialize>(
    value: &T,
    options: EncoderOptions,
) -> Result<String> {
    let mut buf = Vec::new();
    to_writer_with_options(&mut buf, value, options)?;
    Ok(String::from_utf8(buf).map_err(|e| Error::custom(e.to_string()))?)
}

pub fn to_vec<T: ser::Serialize>(value: &T) -> Result<Vec<u8>> {
    to_vec_with_options(value, EncoderOptions::default())
}

pub fn to_vec_with_options<T: ser::Serialize>(
    value: &T,
    options: EncoderOptions,
) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    to_writer_with_options(&mut buf, value, options)?;
    Ok(buf)
}

pub fn to_writer<W: Write, T: ser::Serialize>(writer: W, value: &T) -> Result<()> {
    to_writer_with_options(writer, value, EncoderOptions::default())
}

pub fn to_writer_with_options<W: Write, T: ser::Serialize>(
    writer: W,
    value: &T,
    options: EncoderOptions,
) -> Result<()> {
    let mut serializer = Serializer::new(writer, options);
    value.serialize(&mut serializer)?;
    Ok(())
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<'a, W>;
    type SerializeTuple = SeqSerializer<'a, W>;
    type SerializeTupleStruct = SeqSerializer<'a, W>;
    type SerializeTupleVariant = SeqSerializer<'a, W>;
    type SerializeMap = MapSerializer<'a, W>;
    type SerializeStruct = MapSerializer<'a, W>;
    type SerializeStructVariant = MapSerializer<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_value(&Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_value(&Value::Number(Number::I64(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_value(&Value::Number(Number::U64(v)))
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.serialize_value(&Value::Number(Number::F64(v)))
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_value(&Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized + ser::Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.serialize_value(&Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        use ser::SerializeMap;
        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSerializer {
            serializer: self,
            elements: Vec::new(),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SeqSerializer {
            serializer: self,
            elements: vec![Value::String(variant.to_string())],
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer {
            serializer: self,
            entries: Map::new(),
            current_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(MapSerializer {
            serializer: self,
            entries: Map::new(),
            current_key: Some(variant.to_string()),
        })
    }
}

pub struct SeqSerializer<'a, W> {
    serializer: &'a mut Serializer<W>,
    elements: Vec<Value>,
}

impl<'a, W: Write> ser::SerializeSeq for SeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        let val = to_value(value)?;
        self.elements.push(val);
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.serializer.serialize_value(&Value::Array(self.elements))
    }
}

impl<'a, W: Write> ser::SerializeTuple for SeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W: Write> ser::SerializeTupleStruct for SeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W: Write> ser::SerializeTupleVariant for SeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

pub struct MapSerializer<'a, W> {
    serializer: &'a mut Serializer<W>,
    entries: Map<String, Value>,
    current_key: Option<String>,
}

impl<'a, W: Write> ser::SerializeMap for MapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + ser::Serialize>(&mut self, key: &T) -> Result<()> {
        let key_val = to_value(key)?;
        let key_str = match key_val {
            Value::String(s) => s,
            Value::Number(Number::I64(n)) => n.to_string(),
            Value::Number(Number::U64(n)) => n.to_string(),
            Value::Number(Number::F64(n)) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => return Err(Error::custom("map keys must be strings or primitives")),
        };
        self.current_key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self.current_key.take().ok_or_else(|| Error::custom("serialize_value called without key"))?;
        let val = to_value(value)?;
        self.entries.insert(key, val);
        Ok(())
    }

    fn end(self) -> Result<()> {
        if let Some(variant_key) = self.current_key {
            let mut outer_map = Map::new();
            outer_map.insert(variant_key, Value::Object(self.entries));
            self.serializer.serialize_value(&Value::Object(outer_map))
        } else {
            self.serializer.serialize_value(&Value::Object(self.entries))
        }
    }
}

impl<'a, W: Write> ser::SerializeStruct for MapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        let val = to_value(value)?;
        self.entries.insert(key.to_string(), val);
        Ok(())
    }

    fn end(self) -> Result<()> {
        ser::SerializeMap::end(self)
    }
}

impl<'a, W: Write> ser::SerializeStructVariant for MapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeMap::end(self)
    }
}

fn to_value<T: ser::Serialize + ?Sized>(value: &T) -> Result<Value> {
    struct ValueSerializer;

    impl ser::Serializer for ValueSerializer {
        type Ok = Value;
        type Error = Error;

        type SerializeSeq = ValueSeqSerializer;
        type SerializeTuple = ValueSeqSerializer;
        type SerializeTupleStruct = ValueSeqSerializer;
        type SerializeTupleVariant = ValueSeqSerializer;
        type SerializeMap = ValueMapSerializer;
        type SerializeStruct = ValueMapSerializer;
        type SerializeStructVariant = ValueMapSerializer;

        fn serialize_bool(self, v: bool) -> Result<Value> {
            Ok(Value::Bool(v))
        }

        fn serialize_i8(self, v: i8) -> Result<Value> {
            Ok(Value::Number(Number::I64(v as i64)))
        }

        fn serialize_i16(self, v: i16) -> Result<Value> {
            Ok(Value::Number(Number::I64(v as i64)))
        }

        fn serialize_i32(self, v: i32) -> Result<Value> {
            Ok(Value::Number(Number::I64(v as i64)))
        }

        fn serialize_i64(self, v: i64) -> Result<Value> {
            Ok(Value::Number(Number::I64(v)))
        }

        fn serialize_u8(self, v: u8) -> Result<Value> {
            Ok(Value::Number(Number::U64(v as u64)))
        }

        fn serialize_u16(self, v: u16) -> Result<Value> {
            Ok(Value::Number(Number::U64(v as u64)))
        }

        fn serialize_u32(self, v: u32) -> Result<Value> {
            Ok(Value::Number(Number::U64(v as u64)))
        }

        fn serialize_u64(self, v: u64) -> Result<Value> {
            Ok(Value::Number(Number::U64(v)))
        }

        fn serialize_f32(self, v: f32) -> Result<Value> {
            Ok(Value::Number(Number::F64(v as f64)))
        }

        fn serialize_f64(self, v: f64) -> Result<Value> {
            Ok(Value::Number(Number::F64(v)))
        }

        fn serialize_char(self, v: char) -> Result<Value> {
            Ok(Value::String(v.to_string()))
        }

        fn serialize_str(self, v: &str) -> Result<Value> {
            Ok(Value::String(v.to_string()))
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
            Ok(Value::Array(v.iter().map(|&b| Value::Number(Number::U64(b as u64))).collect()))
        }

        fn serialize_none(self) -> Result<Value> {
            Ok(Value::Null)
        }

        fn serialize_some<T: ?Sized + ser::Serialize>(self, value: &T) -> Result<Value> {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Value> {
            Ok(Value::Null)
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
            Ok(Value::Null)
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Value> {
            Ok(Value::String(variant.to_string()))
        }

        fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Value> {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Value> {
            let mut map = Map::new();
            map.insert(variant.to_string(), to_value(value)?);
            Ok(Value::Object(map))
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
            Ok(ValueSeqSerializer { elements: Vec::new() })
        }

        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
            self.serialize_seq(Some(len))
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct> {
            self.serialize_seq(Some(len))
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant> {
            Ok(ValueSeqSerializer {
                elements: vec![Value::String(variant.to_string())],
            })
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
            Ok(ValueMapSerializer {
                entries: Map::new(),
                current_key: None,
                variant_key: None,
            })
        }

        fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
            self.serialize_map(Some(len))
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant> {
            Ok(ValueMapSerializer {
                entries: Map::new(),
                current_key: None,
                variant_key: Some(variant.to_string()),
            })
        }
    }

    struct ValueSeqSerializer {
        elements: Vec<Value>,
    }

    impl ser::SerializeSeq for ValueSeqSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            self.elements.push(to_value(value)?);
            Ok(())
        }

        fn end(self) -> Result<Value> {
            Ok(Value::Array(self.elements))
        }
    }

    impl ser::SerializeTuple for ValueSeqSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            ser::SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Value> {
            ser::SerializeSeq::end(self)
        }
    }

    impl ser::SerializeTupleStruct for ValueSeqSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            ser::SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Value> {
            ser::SerializeSeq::end(self)
        }
    }

    impl ser::SerializeTupleVariant for ValueSeqSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            ser::SerializeSeq::serialize_element(self, value)
        }

        fn end(self) -> Result<Value> {
            ser::SerializeSeq::end(self)
        }
    }

    struct ValueMapSerializer {
        entries: Map<String, Value>,
        current_key: Option<String>,
        variant_key: Option<String>,
    }

    impl ser::SerializeMap for ValueMapSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_key<T: ?Sized + ser::Serialize>(&mut self, key: &T) -> Result<()> {
            let key_val = to_value(key)?;
            let key_str = match key_val {
                Value::String(s) => s,
                Value::Number(Number::I64(n)) => n.to_string(),
                Value::Number(Number::U64(n)) => n.to_string(),
                Value::Number(Number::F64(n)) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                _ => return Err(Error::custom("map keys must be strings or primitives")),
            };
            self.current_key = Some(key_str);
            Ok(())
        }

        fn serialize_value<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
            let key = self.current_key.take().ok_or_else(|| Error::custom("serialize_value called without key"))?;
            self.entries.insert(key, to_value(value)?);
            Ok(())
        }

        fn end(self) -> Result<Value> {
            if let Some(variant_key) = self.variant_key {
                let mut outer_map = Map::new();
                outer_map.insert(variant_key, Value::Object(self.entries));
                Ok(Value::Object(outer_map))
            } else {
                Ok(Value::Object(self.entries))
            }
        }
    }

    impl ser::SerializeStruct for ValueMapSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_field<T: ?Sized + ser::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()> {
            self.entries.insert(key.to_string(), to_value(value)?);
            Ok(())
        }

        fn end(self) -> Result<Value> {
            ser::SerializeMap::end(self)
        }
    }

    impl ser::SerializeStructVariant for ValueMapSerializer {
        type Ok = Value;
        type Error = Error;

        fn serialize_field<T: ?Sized + ser::Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()> {
            ser::SerializeStruct::serialize_field(self, key, value)
        }

        fn end(self) -> Result<Value> {
            ser::SerializeMap::end(self)
        }
    }

    value.serialize(ValueSerializer)
}
