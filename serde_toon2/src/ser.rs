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
    top_level_keys: std::collections::HashSet<String>,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W, options: EncoderOptions) -> Self {
        let document_delimiter = options.delimiter;
        Serializer {
            writer,
            options,
            depth: 0,
            document_delimiter,
            top_level_keys: std::collections::HashSet::new(),
        }
    }

    fn indent(&self) -> String {
        " ".repeat(self.depth * self.options.indent)
    }

    fn needs_quoting(&self, s: &str, active_delimiter: Delimiter) -> bool {
        if s.is_empty() {
            return true;
        }

        if s.starts_with(|c: char| c.is_whitespace()) || s.ends_with(|c: char| c.is_whitespace()) {
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

        if Self::looks_like_number(s) {
            return true;
        }

        if Self::has_leading_zeros(s) {
            return true;
        }

        false
    }

    #[inline]
    fn looks_like_number(s: &str) -> bool {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return false;
        }

        let mut i = 0;

        // Optional leading minus
        if bytes[i] == b'-' {
            i += 1;
            if i >= bytes.len() {
                return false;
            }
        }

        // Must have at least one digit
        if !bytes[i].is_ascii_digit() {
            return false;
        }

        // Integer part
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }

        if i >= bytes.len() {
            return true; // Just an integer
        }

        // Optional decimal part
        if bytes[i] == b'.' {
            i += 1;
            if i >= bytes.len() {
                return false; // Trailing dot
            }
            if !bytes[i].is_ascii_digit() {
                return false; // No digits after dot
            }
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        if i >= bytes.len() {
            return true; // Number with decimal
        }

        // Optional exponent
        if bytes[i] == b'e' || bytes[i] == b'E' {
            i += 1;
            if i >= bytes.len() {
                return false;
            }
            // Optional sign
            if bytes[i] == b'+' || bytes[i] == b'-' {
                i += 1;
                if i >= bytes.len() {
                    return false;
                }
            }
            // Must have at least one digit
            if !bytes[i].is_ascii_digit() {
                return false;
            }
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        i == bytes.len()
    }

    #[inline]
    fn has_leading_zeros(s: &str) -> bool {
        let bytes = s.as_bytes();
        bytes.len() >= 2 && bytes[0] == b'0' && bytes[1].is_ascii_digit()
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
                    let quoted_key = if self.key_needs_quoting(k) {
                        format!("\"{}\"", self.escape_string(k))
                    } else {
                        k.to_string()
                    };
                    write!(self.writer, "{}{}: null", self.indent(), quoted_key)?;
                } else {
                    write!(self.writer, "null")?;
                }
            }
            Value::Bool(b) => {
                if let Some(k) = key {
                    let quoted_key = if self.key_needs_quoting(k) {
                        format!("\"{}\"", self.escape_string(k))
                    } else {
                        k.to_string()
                    };
                    write!(self.writer, "{}{}: {}", self.indent(), quoted_key, b)?;
                } else {
                    write!(self.writer, "{}", b)?;
                }
            }
            Value::Number(n) => {
                let formatted = self.format_number(n);
                if let Some(k) = key {
                    let quoted_key = if self.key_needs_quoting(k) {
                        format!("\"{}\"", self.escape_string(k))
                    } else {
                        k.to_string()
                    };
                    write!(
                        self.writer,
                        "{}{}: {}",
                        self.indent(),
                        quoted_key,
                        formatted
                    )?;
                } else {
                    write!(self.writer, "{}", formatted)?;
                }
            }
            Value::String(s) => {
                if let Some(k) = key {
                    let quoted_key = if self.key_needs_quoting(k) {
                        format!("\"{}\"", self.escape_string(k))
                    } else {
                        k.to_string()
                    };
                    write!(self.writer, "{}{}: ", self.indent(), quoted_key)?;
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
                write!(
                    self.writer,
                    "{}{}[{}{}]:",
                    self.indent(),
                    k,
                    len,
                    header_delim
                )?;
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
        arr.iter().all(|v| {
            matches!(
                v,
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
            )
        })
    }

    fn is_array_of_arrays(&self, arr: &[Value]) -> bool {
        if arr.is_empty() {
            return false;
        }
        arr.iter()
            .all(|v| matches!(v, Value::Array(inner) if self.is_primitive_array(inner)))
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
                        if !matches!(
                            val,
                            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
                        ) {
                            return Some((false, vec![]));
                        }
                    }

                    if let Some(ref expected_fields) = fields {
                        let keys_set: std::collections::HashSet<_> = item_keys.iter().collect();
                        let expected_set: std::collections::HashSet<_> =
                            expected_fields.iter().collect();
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

        if all_objects && let Some(fields_value) = fields {
            Some((true, fields_value))
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

        if arr.is_empty() {
            if let Some(k) = key {
                write!(
                    self.writer,
                    "{}{}[{}{}]:",
                    self.indent(),
                    k,
                    len,
                    header_delim
                )?;
            } else {
                write!(self.writer, "[{}{}]:", len, header_delim)?;
            }
            return Ok(());
        }

        if let Some(k) = key {
            write!(
                self.writer,
                "{}{}[{}{}]: ",
                self.indent(),
                k,
                len,
                header_delim
            )?;
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
            write!(
                self.writer,
                "{}{}[{}{}]:",
                self.indent(),
                k,
                len,
                header_delim
            )?;
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
            write!(
                self.writer,
                "{}{}[{}{}]{{",
                self.indent(),
                k,
                len,
                header_delim
            )?;
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
            write!(
                self.writer,
                "{}{}[{}{}]:",
                self.indent(),
                k,
                len,
                header_delim
            )?;
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
                    Value::Number(n) => {
                        write!(self.writer, "{}: {}", quoted_key, self.format_number(n))?
                    }
                    Value::String(s) => {
                        write!(self.writer, "{}: ", quoted_key)?;
                        self.write_string(s, self.document_delimiter)?;
                    }
                    Value::Array(arr) => {
                        if self.is_primitive_array(arr) {
                            let len = arr.len();
                            let header_delim = active_delimiter.header_marker();
                            if arr.is_empty() {
                                write!(self.writer, "{}[{}{}]:", quoted_key, len, header_delim)?;
                            } else {
                                write!(self.writer, "{}[{}{}]: ", quoted_key, len, header_delim)?;
                                for (i, val) in arr.iter().enumerate() {
                                    if i > 0 {
                                        write!(self.writer, "{}", active_delimiter.as_str())?;
                                    }
                                    match val {
                                        Value::Null => write!(self.writer, "null")?,
                                        Value::Bool(b) => write!(self.writer, "{}", b)?,
                                        Value::Number(n) => {
                                            write!(self.writer, "{}", self.format_number(n))?
                                        }
                                        Value::String(s) => {
                                            self.write_string(s, active_delimiter)?
                                        }
                                        _ => unreachable!(),
                                    }
                                }
                            }
                        } else if self.is_array_of_arrays(arr) {
                            let len = arr.len();
                            let header_delim = active_delimiter.header_marker();
                            write!(self.writer, "{}[{}{}]:", quoted_key, len, header_delim)?;
                            self.depth += 1;
                            for inner_arr in arr {
                                if let Value::Array(inner) = inner_arr {
                                    write!(self.writer, "\n{}- ", self.indent())?;
                                    self.serialize_primitive_array(inner, None, active_delimiter)?;
                                }
                            }
                            self.depth -= 1;
                        } else if let Some((is_tabular, fields)) = self.detect_tabular(arr) {
                            if is_tabular {
                                let len = arr.len();
                                let header_delim = active_delimiter.header_marker();
                                write!(self.writer, "{}[{}{}]{{", quoted_key, len, header_delim)?;
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
                                                write!(
                                                    self.writer,
                                                    "{}",
                                                    active_delimiter.as_str()
                                                )?;
                                            }
                                            if let Some(val) = map.get(field_name) {
                                                match val {
                                                    Value::Null => write!(self.writer, "null")?,
                                                    Value::Bool(b) => write!(self.writer, "{}", b)?,
                                                    Value::Number(n) => write!(
                                                        self.writer,
                                                        "{}",
                                                        self.format_number(n)
                                                    )?,
                                                    Value::String(s) => {
                                                        self.write_string(s, active_delimiter)?
                                                    }
                                                    _ => unreachable!(),
                                                }
                                            }
                                        }
                                    }
                                }
                                self.depth -= 1;
                            } else {
                                let len = arr.len();
                                let header_delim = active_delimiter.header_marker();
                                write!(self.writer, "{}[{}{}]:", quoted_key, len, header_delim)?;
                                self.depth += 1;
                                for item in arr {
                                    write!(self.writer, "\n{}- ", self.indent())?;
                                    match item {
                                        Value::Null => write!(self.writer, "null")?,
                                        Value::Bool(b) => write!(self.writer, "{}", b)?,
                                        Value::Number(n) => {
                                            write!(self.writer, "{}", self.format_number(n))?
                                        }
                                        Value::String(s) => {
                                            self.write_string(s, active_delimiter)?;
                                        }
                                        Value::Array(inner) => {
                                            self.serialize_primitive_array(
                                                inner,
                                                None,
                                                active_delimiter,
                                            )?;
                                        }
                                        Value::Object(obj) => {
                                            self.serialize_object_as_list_item(
                                                obj,
                                                active_delimiter,
                                            )?;
                                        }
                                    }
                                }
                                self.depth -= 1;
                            }
                        } else {
                            let len = arr.len();
                            let header_delim = active_delimiter.header_marker();
                            write!(self.writer, "{}[{}{}]:", quoted_key, len, header_delim)?;
                            self.depth += 1;
                            for item in arr {
                                write!(self.writer, "\n{}- ", self.indent())?;
                                match item {
                                    Value::Null => write!(self.writer, "null")?,
                                    Value::Bool(b) => write!(self.writer, "{}", b)?,
                                    Value::Number(n) => {
                                        write!(self.writer, "{}", self.format_number(n))?
                                    }
                                    Value::String(s) => {
                                        self.write_string(s, active_delimiter)?;
                                    }
                                    Value::Array(inner) => {
                                        self.serialize_primitive_array(
                                            inner,
                                            None,
                                            active_delimiter,
                                        )?;
                                    }
                                    Value::Object(obj) => {
                                        self.serialize_object_as_list_item(obj, active_delimiter)?;
                                    }
                                }
                            }
                            self.depth -= 1;
                        }
                    }
                    Value::Object(nested) => {
                        write!(self.writer, "{}:", quoted_key)?;
                        self.depth += 1;
                        for (nested_key, nested_val) in nested {
                            writeln!(self.writer)?;
                            self.serialize_value_with_key(
                                nested_val,
                                Some(nested_key),
                                self.document_delimiter,
                            )?;
                        }
                        self.depth -= 1;
                    }
                }
                first = false;
            } else {
                let quoted_key = if self.key_needs_quoting(key) {
                    format!("\"{}\"", self.escape_string(key))
                } else {
                    key.clone()
                };

                let extra_indent = " ".repeat(self.options.indent);
                let field_indent = format!("{}{}", self.indent(), extra_indent);

                match value {
                    Value::Null => write!(self.writer, "\n{}{}: null", field_indent, quoted_key)?,
                    Value::Bool(b) => {
                        write!(self.writer, "\n{}{}: {}", field_indent, quoted_key, b)?
                    }
                    Value::Number(n) => write!(
                        self.writer,
                        "\n{}{}: {}",
                        field_indent,
                        quoted_key,
                        self.format_number(n)
                    )?,
                    Value::String(s) => {
                        write!(self.writer, "\n{}{}: ", field_indent, quoted_key)?;
                        self.write_string(s, self.document_delimiter)?;
                    }
                    Value::Array(arr) => {
                        writeln!(self.writer)?;
                        self.depth += 1;
                        self.serialize_array(arr, Some(key), active_delimiter)?;
                        self.depth -= 1;
                    }
                    Value::Object(nested) => {
                        write!(self.writer, "\n{}{}:", field_indent, quoted_key)?;
                        self.depth += 2;
                        for (nested_key, nested_val) in nested {
                            writeln!(self.writer)?;
                            self.serialize_value_with_key(
                                nested_val,
                                Some(nested_key),
                                self.document_delimiter,
                            )?;
                        }
                        self.depth -= 2;
                    }
                }
            }
        }

        Ok(())
    }

    fn key_needs_quoting(&self, key: &str) -> bool {
        if key.is_empty() {
            return true;
        }

        if !Self::is_valid_unquoted_key(key) {
            return true;
        }

        if key.contains('\n')
            || key.contains('\r')
            || key.contains('\t')
            || key.contains('\\')
            || key.contains('"')
        {
            return true;
        }

        false
    }

    #[inline]
    fn is_valid_unquoted_key(s: &str) -> bool {
        let bytes = s.as_bytes();
        if bytes.is_empty() {
            return false;
        }

        // First character must be A-Za-z_
        let first = bytes[0];
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return false;
        }

        // Fast path for remaining characters using lookup table
        static VALID_KEY_CHARS: [bool; 256] = {
            let mut table = [false; 256];
            let mut i = 0;
            while i < 256 {
                table[i] = (i >= b'A' as usize && i <= b'Z' as usize)
                    || (i >= b'a' as usize && i <= b'z' as usize)
                    || (i >= b'0' as usize && i <= b'9' as usize)
                    || i == b'_' as usize
                    || i == b'.' as usize;
                i += 1;
            }
            table
        };

        // Check remaining bytes using lookup table
        for &byte in &bytes[1..] {
            if !VALID_KEY_CHARS[byte as usize] {
                return false;
            }
        }

        true
    }

    fn try_fold_object(
        &self,
        obj: &Map<String, Value>,
        num_segments: usize,
    ) -> Option<(String, Value)> {
        use crate::options::KeyFolding;

        // Key folding must be enabled
        if self.options.key_folding != KeyFolding::Safe {
            return None;
        }

        // Only fold single-key objects
        if obj.len() != 1 {
            return None;
        }

        let (key, value) = obj.iter().next().unwrap();

        // In safe mode, don't fold keys that need quoting
        if self.key_needs_quoting(key) {
            return None;
        }

        // Count the current segment
        let new_num_segments = num_segments + 1;

        // Check flatten depth limit - have we reached the maximum?
        if new_num_segments > self.options.flatten_depth {
            // We've exceeded the limit, stop folding
            return None;
        }

        // Try to continue folding if the value is a single-key object
        if let Value::Object(nested_obj) = value {
            // Only recurse if we haven't reached the limit yet
            if new_num_segments < self.options.flatten_depth
                && let Some((nested_path, final_value)) =
                    self.try_fold_object(nested_obj, new_num_segments)
            {
                let full_path = format!("{}.{}", key, nested_path);
                return Some((full_path, final_value));
            }
            // If we've reached the limit exactly, return just this key
            // without trying to fold further
        }

        // This is the end of the foldable chain (either we hit the limit or the value isn't an object)
        Some((key.clone(), value.clone()))
    }

    fn serialize_object(&mut self, obj: &Map<String, Value>, key: Option<&str>) -> Result<()> {
        if obj.is_empty() {
            if let Some(k) = key {
                let quoted_key = if self.key_needs_quoting(k) {
                    format!("\"{}\"", self.escape_string(k))
                } else {
                    k.to_string()
                };
                write!(self.writer, "{}{}:", self.indent(), quoted_key)?;
            }
            return Ok(());
        }

        if let Some(k) = key {
            let quoted_key = if self.key_needs_quoting(k) {
                format!("\"{}\"", self.escape_string(k))
            } else {
                k.to_string()
            };
            write!(self.writer, "{}{}:", self.indent(), quoted_key)?;
            self.depth += 1;

            for (obj_key, obj_val) in obj {
                writeln!(self.writer)?;
                self.serialize_value_with_key(obj_val, Some(obj_key), self.document_delimiter)?;
            }
            self.depth -= 1;
        } else {
            // We're at the top level - collect all keys for collision detection
            self.top_level_keys = obj.keys().cloned().collect();

            for (i, (obj_key, obj_val)) in obj.iter().enumerate() {
                if i > 0 {
                    writeln!(self.writer)?;
                }

                // Try to fold if it's an object
                if let Value::Object(nested_obj) = obj_val {
                    // In safe mode, don't fold if the parent key needs quoting
                    if !self.key_needs_quoting(obj_key) {
                        // Start with 1 to account for the current key (obj_key)
                        if let Some((folded_path, final_value)) =
                            self.try_fold_object(nested_obj, 1)
                        {
                            let full_key = format!("{}.{}", obj_key, folded_path);
                            // Check for collision with sibling keys at this (top) level
                            if !self.top_level_keys.contains(&full_key) {
                                self.serialize_value_with_key(
                                    &final_value,
                                    Some(&full_key),
                                    self.document_delimiter,
                                )?;
                                continue;
                            }
                        }
                    }
                }

                self.serialize_value_with_key(obj_val, Some(obj_key), self.document_delimiter)?;
            }
        }

        Ok(())
    }
}

/// Serializes a value to a TOON string using default options.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use serde_toon2::to_string;
///
/// #[derive(Serialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let person = Person {
///     name: "Ada".to_string(),
///     age: 42,
/// };
///
/// let toon = to_string(&person).unwrap();
/// assert_eq!(toon, "name: Ada\nage: 42");
/// ```
pub fn to_string<T: ser::Serialize>(value: &T) -> Result<String> {
    to_string_with_options(value, EncoderOptions::default())
}

/// Serializes a value to a TOON string with custom options.
///
/// # Examples
///
/// ```
/// use serde_toon2::{to_string_with_options, EncoderOptions, Delimiter};
///
/// let data = vec!["a", "b", "c"];
///
/// let opts = EncoderOptions {
///     delimiter: Delimiter::Pipe,
///     ..Default::default()
/// };
///
/// let toon = to_string_with_options(&data, opts).unwrap();
/// assert!(toon.contains("|"));
/// ```
pub fn to_string_with_options<T: ser::Serialize>(
    value: &T,
    options: EncoderOptions,
) -> Result<String> {
    let mut buf = Vec::new();
    to_writer_with_options(&mut buf, value, options)?;
    String::from_utf8(buf).map_err(|e| Error::custom(e.to_string()))
}

/// Serializes a value to a TOON byte vector using default options.
///
/// # Examples
///
/// ```
/// use serde_toon2::to_vec;
///
/// let data = vec![1, 2, 3];
/// let bytes = to_vec(&data).unwrap();
/// let toon = String::from_utf8(bytes).unwrap();
/// assert_eq!(toon, "[3]: 1,2,3");
/// ```
pub fn to_vec<T: ser::Serialize>(value: &T) -> Result<Vec<u8>> {
    to_vec_with_options(value, EncoderOptions::default())
}

/// Serializes a value to a TOON byte vector with custom options.
///
/// # Examples
///
/// ```
/// use serde_toon2::{to_vec_with_options, EncoderOptions};
///
/// let data = vec![1, 2, 3];
/// let opts = EncoderOptions {
///     indent: 4,
///     ..Default::default()
/// };
/// let bytes = to_vec_with_options(&data, opts).unwrap();
/// assert!(!bytes.is_empty());
/// ```
pub fn to_vec_with_options<T: ser::Serialize>(
    value: &T,
    options: EncoderOptions,
) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    to_writer_with_options(&mut buf, value, options)?;
    Ok(buf)
}

/// Serializes a value to TOON format and writes it to the given writer using default options.
///
/// # Examples
///
/// ```
/// use serde_toon2::to_writer;
/// use std::io::Cursor;
///
/// let data = vec!["x", "y", "z"];
/// let mut buffer = Cursor::new(Vec::new());
/// to_writer(&mut buffer, &data).unwrap();
///
/// let toon = String::from_utf8(buffer.into_inner()).unwrap();
/// assert_eq!(toon, "[3]: x,y,z");
/// ```
pub fn to_writer<W: Write, T: ser::Serialize>(writer: W, value: &T) -> Result<()> {
    to_writer_with_options(writer, value, EncoderOptions::default())
}

/// Serializes a value to TOON format and writes it to the given writer with custom options.
///
/// # Examples
///
/// ```
/// use serde_toon2::{to_writer_with_options, EncoderOptions, Delimiter};
/// use std::io::Cursor;
///
/// let data = vec!["x", "y", "z"];
/// let opts = EncoderOptions {
///     delimiter: Delimiter::Tab,
///     ..Default::default()
/// };
///
/// let mut buffer = Cursor::new(Vec::new());
/// to_writer_with_options(&mut buffer, &data, opts).unwrap();
///
/// let toon = String::from_utf8(buffer.into_inner()).unwrap();
/// assert!(toon.contains("\t"));
/// ```
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
        self.serializer
            .serialize_value(&Value::Array(self.elements))
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
        let key = self
            .current_key
            .take()
            .ok_or_else(|| Error::custom("serialize_value called without key"))?;
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
            self.serializer
                .serialize_value(&Value::Object(self.entries))
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
            Ok(Value::Array(
                v.iter()
                    .map(|&b| Value::Number(Number::U64(b as u64)))
                    .collect(),
            ))
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
            Ok(ValueSeqSerializer {
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

        fn serialize_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct> {
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
            let key = self
                .current_key
                .take()
                .ok_or_else(|| Error::custom("serialize_value called without key"))?;
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
