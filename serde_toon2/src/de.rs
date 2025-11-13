use crate::error::{Error, ErrorKind, Result};
use crate::options::{Delimiter, DecoderOptions, PathExpansion};
use crate::value::{Map, Number, Value};
use serde::de;
use serde::forward_to_deserialize_any;
use std::io::Read;

#[derive(Debug, Clone)]
struct Line {
    content: String,
    depth: usize,
    line_number: usize,
    original: String,
}

#[derive(Debug, Clone)]
struct ArrayHeader {
    key: Option<String>,
    length: usize,
    delimiter: Delimiter,
    fields: Option<Vec<String>>,
}

#[allow(dead_code)]
pub struct Deserializer<'de> {
    input: &'de str,
    options: DecoderOptions,
    current_line: usize,
    current_column: usize,
}

impl<'de> Deserializer<'de> {
    pub fn new(input: &'de str, options: DecoderOptions) -> Self {
        Deserializer {
            input,
            options,
            current_line: 1,
            current_column: 1,
        }
    }
}

pub fn from_str<'a, T: de::Deserialize<'a>>(s: &'a str) -> Result<T> {
    from_str_with_options(s, DecoderOptions::default())
}

pub fn from_str_with_options<'a, T: de::Deserialize<'a>>(
    s: &'a str,
    options: DecoderOptions,
) -> Result<T> {
    let lines = tokenize_lines(s, &options)?;
    let value = parse_root(&lines, &options)?;
    let value = if options.expand_paths != PathExpansion::Off {
        expand_paths(value, &options)?
    } else {
        value
    };

    T::deserialize(value)
}

pub fn from_slice<'a, T: de::Deserialize<'a>>(v: &'a [u8]) -> Result<T> {
    from_slice_with_options(v, DecoderOptions::default())
}

pub fn from_slice_with_options<'a, T: de::Deserialize<'a>>(
    v: &'a [u8],
    options: DecoderOptions,
) -> Result<T> {
    let s = std::str::from_utf8(v).map_err(|e| Error::custom(e.to_string()))?;
    from_str_with_options(s, options)
}

pub fn from_reader<R: Read, T: de::DeserializeOwned>(rdr: R) -> Result<T> {
    from_reader_with_options(rdr, DecoderOptions::default())
}

pub fn from_reader_with_options<R: Read, T: de::DeserializeOwned>(
    mut rdr: R,
    options: DecoderOptions,
) -> Result<T> {
    let mut buf = String::new();
    rdr.read_to_string(&mut buf)?;
    from_str_with_options(&buf, options)
}

fn tokenize_lines(input: &str, options: &DecoderOptions) -> Result<Vec<Line>> {
    let mut lines = Vec::new();
    let indent_size = options.indent;

    for (line_number, line_str) in input.lines().enumerate() {
        let line_number = line_number + 1;

        let leading_spaces = line_str.chars().take_while(|&c| c == ' ').count();

        if options.strict {
            // Check for tabs in the leading whitespace (before any non-whitespace)
            let leading_whitespace: String = line_str.chars()
                .take_while(|&c| c.is_whitespace())
                .collect();
            if leading_whitespace.contains('\t') {
                return Err(Error::new(
                    ErrorKind::IndentationError,
                    "Tabs are not allowed in indentation",
                ).with_location(line_number, 1));
            }

            // Only check non-empty lines for indentation multiples
            if !line_str.trim().is_empty() && leading_spaces % indent_size != 0 {
                return Err(Error::new(
                    ErrorKind::IndentationError,
                    format!("Indentation must be a multiple of {}", indent_size),
                ).with_location(line_number, 1));
            }
        }

        let depth = leading_spaces / indent_size;
        // Convert character count to byte position for slicing
        let byte_pos = line_str.char_indices()
            .nth(leading_spaces)
            .map(|(pos, _)| pos)
            .unwrap_or(line_str.len());
        let content = line_str[byte_pos..].to_string();

        if !content.is_empty() {
            lines.push(Line {
                content,
                depth,
                line_number,
                original: line_str.to_string(),
            });
        }
    }

    Ok(lines)
}

fn parse_root(lines: &[Line], options: &DecoderOptions) -> Result<Value> {
    if lines.is_empty() {
        return Ok(Value::Object(Map::new()));
    }

    let first = &lines[0];
    if first.depth != 0 {
        return Err(Error::new(
            ErrorKind::InvalidSyntax,
            "First line must be at depth 0",
        ).with_location(first.line_number, 1));
    }

    if is_array_header(&first.content) {
        if let Some(header) = try_parse_array_header(&first.content)? {
            if header.key.is_none() {
                let mut cursor = 0;
                return parse_value(lines, &mut cursor, 0, Delimiter::Comma, options);
            }
        }
    }

    // Check if it's a single primitive value (no colon outside quotes)
    if lines.len() == 1 {
        let has_unquoted_colon = has_colon_outside_quotes(&first.content);
        if !has_unquoted_colon {
            return parse_primitive(&first.content, first.line_number);
        }
    }

    parse_object(lines, 0, options)
}

fn has_colon_outside_quotes(content: &str) -> bool {
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in content.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_quotes {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if ch == ':' && !in_quotes {
            return true;
        }
    }

    false
}

fn find_colon_outside_quotes(content: &str) -> Option<usize> {
    let mut in_quotes = false;
    let mut escape_next = false;

    for (i, ch) in content.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_quotes {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if ch == ':' && !in_quotes {
            return Some(i);
        }
    }

    None
}

fn parse_object(lines: &[Line], start_idx: usize, options: &DecoderOptions) -> Result<Value> {
    let mut obj = Map::new();
    let mut i = start_idx;

    while i < lines.len() {
        let line = &lines[i];

        if line.depth != 0 {
            break;
        }

        if let Some(header) = try_parse_array_header(&line.content)? {
            let key = header.key.clone().unwrap_or_default();
            let colon_pos = find_colon_outside_quotes(&line.content).unwrap();
            let value_part = line.content[colon_pos + 1..].trim_start();

            if !value_part.is_empty() {
                let values = parse_inline_array(value_part, header.delimiter, header.length, line.line_number)?;
                obj.insert(key, Value::Array(values));
            } else {
                i += 1;
                let value = parse_array_body(lines, &mut i, line.depth, header, options)?;
                obj.insert(key, value);
                continue;
            }
        } else {
            let (key, value_part) = parse_key_value_line(&line.content, line.line_number)?;

            if value_part.is_empty() {
                let nested_start = i + 1;
                let nested_depth = line.depth + 1;

                let mut nested_end = nested_start;
                while nested_end < lines.len() && lines[nested_end].depth >= nested_depth {
                    nested_end += 1;
                }

                if nested_end > nested_start {
                    let nested_lines: Vec<Line> = lines[nested_start..nested_end]
                        .iter()
                        .map(|l| Line {
                            content: l.content.clone(),
                            depth: l.depth - nested_depth,
                            line_number: l.line_number,
                            original: l.original.clone(),
                        })
                        .collect();

                    let value = parse_object(&nested_lines, 0, options)?;
                    obj.insert(key, value);
                    i = nested_end;
                    continue;
                } else {
                    obj.insert(key, Value::Object(Map::new()));
                }
            } else {
                let value = parse_primitive(value_part, line.line_number)?;
                obj.insert(key, value);
            }
        }

        i += 1;
    }

    Ok(Value::Object(obj))
}

fn parse_value(
    lines: &[Line],
    cursor: &mut usize,
    parent_depth: usize,
    _parent_delimiter: Delimiter,
    options: &DecoderOptions,
) -> Result<Value> {
    if *cursor >= lines.len() {
        return Err(Error::new(ErrorKind::InvalidSyntax, "Unexpected end of input"));
    }

    let line = &lines[*cursor];

    if let Some(header) = try_parse_array_header(&line.content)? {
        let value_start = line.content.find(':').unwrap() + 1;
        let value_part = line.content[value_start..].trim_start();

        if !value_part.is_empty() {
            let values = parse_inline_array(value_part, header.delimiter, header.length, line.line_number)?;
            *cursor += 1;
            Ok(Value::Array(values))
        } else {
            *cursor += 1;
            parse_array_body(lines, cursor, parent_depth, header, options)
        }
    } else if line.content.contains(':') {
        let obj = parse_object_at_depth(lines, cursor, parent_depth, options)?;
        Ok(Value::Object(obj))
    } else {
        let value = parse_primitive(&line.content, line.line_number)?;
        *cursor += 1;
        Ok(value)
    }
}

fn parse_object_at_depth(
    lines: &[Line],
    cursor: &mut usize,
    depth: usize,
    options: &DecoderOptions,
) -> Result<Map<String, Value>> {
    let mut obj = Map::new();

    while *cursor < lines.len() && lines[*cursor].depth == depth {
        let line = &lines[*cursor];

        if let Some(header) = try_parse_array_header(&line.content)? {
            let key = header.key.clone().unwrap_or_default();
            let colon_pos = find_colon_outside_quotes(&line.content).unwrap();
            let value_part = line.content[colon_pos + 1..].trim_start();

            if !value_part.is_empty() {
                let values = parse_inline_array(value_part, header.delimiter, header.length, line.line_number)?;
                obj.insert(key, Value::Array(values));
                *cursor += 1;
            } else {
                *cursor += 1;
                let value = parse_array_body(lines, cursor, depth, header, options)?;
                obj.insert(key, value);
            }
        } else {
            let (key, value_part) = parse_key_value_line(&line.content, line.line_number)?;

            if value_part.is_empty() {
                *cursor += 1;
                let nested_depth = depth + 1;

                if *cursor < lines.len() && lines[*cursor].depth == nested_depth {
                    let nested_obj = parse_object_at_depth(lines, cursor, nested_depth, options)?;
                    obj.insert(key, Value::Object(nested_obj));
                } else {
                    obj.insert(key, Value::Object(Map::new()));
                }
            } else {
                let value = parse_primitive(value_part, line.line_number)?;
                obj.insert(key, value);
                *cursor += 1;
            }
        }
    }

    Ok(obj)
}

fn parse_array_body(
    lines: &[Line],
    cursor: &mut usize,
    parent_depth: usize,
    header: ArrayHeader,
    options: &DecoderOptions,
) -> Result<Value> {
    let item_depth = parent_depth + 1;

    if header.fields.is_some() {
        parse_tabular_array(lines, cursor, item_depth, header, options)
    } else {
        parse_list_array(lines, cursor, item_depth, header, options)
    }
}

fn parse_tabular_array(
    lines: &[Line],
    cursor: &mut usize,
    item_depth: usize,
    header: ArrayHeader,
    options: &DecoderOptions,
) -> Result<Value> {
    let fields = header.fields.as_ref().unwrap();
    let mut rows = Vec::new();
    let mut prev_line_number: Option<usize> = None;

    while *cursor < lines.len() && lines[*cursor].depth == item_depth {
        let line = &lines[*cursor];

        if is_tabular_row(&line.content, header.delimiter) {
            if options.strict {
                if let Some(prev) = prev_line_number {
                    if line.line_number > prev + 1 {
                        return Err(Error::new(
                            ErrorKind::InvalidSyntax,
                            "Blank lines are not allowed inside arrays",
                        ).with_location(line.line_number, 1));
                    }
                }
            }
            prev_line_number = Some(line.line_number);
            let values = parse_delimited_values(&line.content, header.delimiter, line.line_number)?;

            if values.len() != fields.len() {
                return Err(Error::new(
                    ErrorKind::WidthMismatch,
                    format!("Expected {} values, got {}", fields.len(), values.len()),
                ).with_location(line.line_number, 1));
            }

            let mut obj = Map::new();
            for (i, field) in fields.iter().enumerate() {
                let value = if i < values.len() {
                    parse_primitive(&values[i], line.line_number)?
                } else {
                    Value::Null
                };
                obj.insert(field.clone(), value);
            }
            rows.push(Value::Object(obj));
            *cursor += 1;
        } else {
            break;
        }
    }

    if rows.len() != header.length {
        return Err(Error::new(
            ErrorKind::CountMismatch,
            format!("Expected {} rows, got {}", header.length, rows.len()),
        ));
    }

    Ok(Value::Array(rows))
}

fn parse_list_array(
    lines: &[Line],
    cursor: &mut usize,
    item_depth: usize,
    header: ArrayHeader,
    options: &DecoderOptions,
) -> Result<Value> {
    let mut items = Vec::new();
    let mut prev_line_number: Option<usize> = None;

    while *cursor < lines.len() && lines[*cursor].depth == item_depth {
        let line = &lines[*cursor];

        if !line.content.starts_with('-') {
            break;
        }

        if options.strict {
            if let Some(prev) = prev_line_number {
                if line.line_number > prev + 1 {
                    return Err(Error::new(
                        ErrorKind::InvalidSyntax,
                        "Blank lines are not allowed inside arrays",
                    ).with_location(line.line_number, 1));
                }
            }
        }
        prev_line_number = Some(line.line_number);

        let item_content = if line.content.starts_with("- ") {
            &line.content[2..]
        } else if line.content == "-" {
            ""
        } else {
            break;
        };

        if item_content.is_empty() {
            items.push(Value::Object(Map::new()));
            *cursor += 1;
            continue;
        } else if let Some(inner_header) = try_parse_array_header(item_content)? {
            // If the array header has a key, it should be treated as an object field
            // e.g., "tags[3]: a,b,c" should become {"tags": ["a", "b", "c"]}
            // Only if key is None should we treat it as a root array
            if inner_header.key.is_some() {
                // Fall through to object parsing
            } else {
                // Root array without key - parse as direct array value
                let value_start = item_content.find(':').unwrap() + 1;
                let value_part = item_content[value_start..].trim_start();

                if !value_part.is_empty() {
                    let values = parse_inline_array(value_part, inner_header.delimiter, inner_header.length, line.line_number)?;
                    items.push(Value::Array(values));
                    *cursor += 1;
                    continue;
                } else {
                    *cursor += 1;
                    let value = parse_array_body(lines, cursor, item_depth, inner_header, options)?;
                    items.push(value);
                    continue;
                }
            }
        }

        if item_content.contains(':') {
            let (key, value_part) = parse_key_value_line(item_content, line.line_number)?;
            let mut obj = Map::new();

            if let Some(arr_header) = try_parse_array_header(item_content)? {
                // Use the key from the array header, not the parsed key which includes brackets
                let array_key = arr_header.key.clone().unwrap_or(key.clone());
                if !value_part.is_empty() {
                    let values = parse_inline_array(value_part, arr_header.delimiter, arr_header.length, line.line_number)?;
                    obj.insert(array_key, Value::Array(values));
                    *cursor += 1;
                } else {
                    *cursor += 1;
                    let value = parse_array_body(lines, cursor, item_depth, arr_header, options)?;
                    obj.insert(array_key, value);
                }

                // Check for sibling fields at item_depth + 1
                let sibling_depth = item_depth + 1;
                while *cursor < lines.len() && lines[*cursor].depth == sibling_depth {
                    let sibling_line = &lines[*cursor];
                    let (sib_key, sib_value_part) = parse_key_value_line(&sibling_line.content, sibling_line.line_number)?;

                    if let Some(sib_header) = try_parse_array_header(&sibling_line.content)? {
                        let array_key = sib_header.key.clone().unwrap_or(sib_key.clone());
                        if !sib_value_part.is_empty() {
                            let values = parse_inline_array(sib_value_part, sib_header.delimiter, sib_header.length, sibling_line.line_number)?;
                            obj.insert(array_key, Value::Array(values));
                            *cursor += 1;
                        } else {
                            *cursor += 1;
                            let value = parse_array_body(lines, cursor, sibling_depth - 1, sib_header, options)?;
                            obj.insert(array_key, value);
                        }
                    } else if sib_value_part.is_empty() {
                        *cursor += 1;
                        let nested_obj = parse_object_at_depth(lines, cursor, sibling_depth + 1, options)?;
                        obj.insert(sib_key, Value::Object(nested_obj));
                    } else {
                        let value = parse_primitive(sib_value_part, sibling_line.line_number)?;
                        obj.insert(sib_key, value);
                        *cursor += 1;
                    }
                }
            } else if value_part.is_empty() {
                *cursor += 1;
                let nested_depth = item_depth + 2;

                if *cursor < lines.len() && lines[*cursor].depth == nested_depth {
                    let nested_obj = parse_object_at_depth(lines, cursor, nested_depth, options)?;
                    obj.insert(key, Value::Object(nested_obj));
                } else {
                    obj.insert(key, Value::Object(Map::new()));
                }

                let sibling_depth = item_depth + 1;
                while *cursor < lines.len() && lines[*cursor].depth == sibling_depth {
                    let sibling_line = &lines[*cursor];
                    let (sib_key, sib_value_part) = parse_key_value_line(&sibling_line.content, sibling_line.line_number)?;

                    if let Some(sib_header) = try_parse_array_header(&sibling_line.content)? {
                        let array_key = sib_header.key.clone().unwrap_or(sib_key.clone());
                        if !sib_value_part.is_empty() {
                            let values = parse_inline_array(sib_value_part, sib_header.delimiter, sib_header.length, sibling_line.line_number)?;
                            obj.insert(array_key, Value::Array(values));
                            *cursor += 1;
                        } else {
                            *cursor += 1;
                            let value = parse_array_body(lines, cursor, sibling_depth - 1, sib_header, options)?;
                            obj.insert(array_key, value);
                        }
                    } else if sib_value_part.is_empty() {
                        *cursor += 1;
                        let nested_obj = parse_object_at_depth(lines, cursor, sibling_depth + 1, options)?;
                        obj.insert(sib_key, Value::Object(nested_obj));
                    } else {
                        let value = parse_primitive(sib_value_part, sibling_line.line_number)?;
                        obj.insert(sib_key, value);
                        *cursor += 1;
                    }
                }
            } else {
                let value = parse_primitive(value_part, line.line_number)?;
                obj.insert(key, value);
                *cursor += 1;

                let sibling_depth = item_depth + 1;
                while *cursor < lines.len() && lines[*cursor].depth == sibling_depth {
                    let sibling_line = &lines[*cursor];
                    let (sib_key, sib_value_part) = parse_key_value_line(&sibling_line.content, sibling_line.line_number)?;

                    if let Some(sib_header) = try_parse_array_header(&sibling_line.content)? {
                        let array_key = sib_header.key.clone().unwrap_or(sib_key.clone());
                        if !sib_value_part.is_empty() {
                            let values = parse_inline_array(sib_value_part, sib_header.delimiter, sib_header.length, sibling_line.line_number)?;
                            obj.insert(array_key, Value::Array(values));
                            *cursor += 1;
                        } else {
                            *cursor += 1;
                            let value = parse_array_body(lines, cursor, sibling_depth - 1, sib_header, options)?;
                            obj.insert(array_key, value);
                        }
                    } else if sib_value_part.is_empty() {
                        *cursor += 1;
                        let nested_obj = parse_object_at_depth(lines, cursor, sibling_depth + 1, options)?;
                        obj.insert(sib_key, Value::Object(nested_obj));
                    } else {
                        let value = parse_primitive(sib_value_part, sibling_line.line_number)?;
                        obj.insert(sib_key, value);
                        *cursor += 1;
                    }
                }
            }

            items.push(Value::Object(obj));
        } else {
            let value = parse_primitive(item_content, line.line_number)?;
            items.push(value);
            *cursor += 1;
        }
    }

    if items.len() != header.length {
        return Err(Error::new(
            ErrorKind::CountMismatch,
            format!("Expected {} items, got {}", header.length, items.len()),
        ));
    }

    Ok(Value::Array(items))
}

fn parse_inline_array(
    content: &str,
    delimiter: Delimiter,
    expected_count: usize,
    line_number: usize,
) -> Result<Vec<Value>> {
    let values_str = parse_delimited_values(content, delimiter, line_number)?;
    let mut values = Vec::new();

    for val_str in values_str {
        values.push(parse_primitive(&val_str, line_number)?);
    }

    if values.len() != expected_count {
        return Err(Error::new(
            ErrorKind::CountMismatch,
            format!("Expected {} values, got {}", expected_count, values.len()),
        ).with_location(line_number, 1));
    }

    Ok(values)
}

fn parse_delimited_values(content: &str, delimiter: Delimiter, _line_number: usize) -> Result<Vec<String>> {
    let delim_char = delimiter.as_char();
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if escape_next {
            current.push(ch);
            escape_next = false;
        } else if ch == '\\' && in_quotes {
            escape_next = true;
            current.push(ch);
        } else if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
        } else if ch == delim_char && !in_quotes {
            values.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }

    values.push(current.trim().to_string());

    Ok(values)
}

fn parse_key_value_line(content: &str, line_number: usize) -> Result<(String, &str)> {
    let mut in_quotes = false;
    let mut escape_next = false;
    let mut colon_pos = None;

    for (i, ch) in content.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_quotes {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if ch == ':' && !in_quotes {
            colon_pos = Some(i);
            break;
        }
    }

    let colon_pos = colon_pos.ok_or_else(|| {
        Error::new(ErrorKind::MissingColon, "Missing colon after key")
            .with_location(line_number, 1)
    })?;

    let key_part = content[..colon_pos].trim();
    let value_part = if colon_pos + 1 < content.len() {
        &content[colon_pos + 1..]
    } else {
        ""
    };

    let key = if key_part.starts_with('"') && key_part.ends_with('"') {
        let unescaped = unescape_string(&key_part[1..key_part.len() - 1], line_number)?;
        // Mark quoted keys with a null byte prefix so path expansion can skip them
        if unescaped.contains('.') {
            format!("\x00{}", unescaped)
        } else {
            unescaped
        }
    } else {
        key_part.to_string()
    };

    let value_part = value_part.trim_start();

    Ok((key, value_part))
}

fn is_array_header(content: &str) -> bool {
    content.contains('[') && content.contains(']') && content.contains(':')
}

fn try_parse_array_header(content: &str) -> Result<Option<ArrayHeader>> {
    if !content.contains('[') || !content.contains(']') || !content.contains(':') {
        return Ok(None);
    }

    let mut in_quotes = false;
    let mut escape_next = false;
    let mut bracket_start = None;
    let mut bracket_end = None;

    for (i, ch) in content.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_quotes {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if !in_quotes {
            if ch == '[' && bracket_start.is_none() {
                bracket_start = Some(i);
            } else if ch == ']' && bracket_start.is_some() && bracket_end.is_none() {
                bracket_end = Some(i);
            }
        }
    }

    if bracket_start.is_none() || bracket_end.is_none() {
        return Ok(None);
    }

    let bracket_start = bracket_start.unwrap();
    let bracket_end = bracket_end.unwrap();

    let key_part = if bracket_start > 0 {
        let key_str = content[..bracket_start].trim();
        if key_str.is_empty() {
            None
        } else if key_str.starts_with('"') && key_str.ends_with('"') {
            Some(key_str[1..key_str.len() - 1].to_string())
        } else {
            Some(key_str.to_string())
        }
    } else {
        None
    };

    let bracket_content = &content[bracket_start + 1..bracket_end];

    let (length_str, delimiter) = if bracket_content.ends_with('\t') {
        (&bracket_content[..bracket_content.len() - 1], Delimiter::Tab)
    } else if bracket_content.ends_with('|') {
        (&bracket_content[..bracket_content.len() - 1], Delimiter::Pipe)
    } else {
        (bracket_content, Delimiter::Comma)
    };

    let length = length_str.parse::<usize>().map_err(|_| {
        Error::new(ErrorKind::InvalidHeader, format!("Invalid array length: {}", length_str))
    })?;

    let after_bracket = &content[bracket_end + 1..].trim_start();

    let fields = if after_bracket.starts_with('{') {
        let mut close_brace_pos = None;
        let mut in_quotes = false;
        let mut escape_next = false;

        for (i, ch) in after_bracket.chars().enumerate().skip(1) {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_quotes {
                escape_next = true;
                continue;
            }

            if ch == '"' {
                in_quotes = !in_quotes;
                continue;
            }

            if ch == '}' && !in_quotes {
                close_brace_pos = Some(i);
                break;
            }
        }

        if let Some(close_brace) = close_brace_pos {
            let fields_content = &after_bracket[1..close_brace];
            let field_strings = parse_delimited_values(fields_content, delimiter, 0)?;
            let mut fields = Vec::new();

            for field_str in field_strings {
                let field = if field_str.starts_with('"') && field_str.ends_with('"') {
                    unescape_string(&field_str[1..field_str.len() - 1], 0)?
                } else {
                    field_str
                };
                fields.push(field);
            }

            Some(fields)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Some(ArrayHeader {
        key: key_part,
        length,
        delimiter,
        fields,
    }))
}

fn is_tabular_row(content: &str, delimiter: Delimiter) -> bool {
    let delim_char = delimiter.as_char();
    let mut in_quotes = false;
    let mut escape_next = false;
    let mut has_delimiter = false;
    let mut has_colon = false;
    let mut first_delim_pos = None;
    let mut first_colon_pos = None;

    for (i, ch) in content.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_quotes {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if !in_quotes {
            if ch == delim_char && !has_delimiter {
                has_delimiter = true;
                first_delim_pos = Some(i);
            }
            if ch == ':' && !has_colon {
                has_colon = true;
                first_colon_pos = Some(i);
            }
        }
    }

    if !has_colon {
        return true;
    }

    if !has_delimiter {
        return false;
    }

    match (first_delim_pos, first_colon_pos) {
        (Some(d), Some(c)) => d < c,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => true,
    }
}

fn parse_primitive(content: &str, line_number: usize) -> Result<Value> {
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return Ok(Value::String(String::new()));
    }

    if trimmed.starts_with('"') {
        if !trimmed.ends_with('"') || trimmed.len() < 2 {
            return Err(Error::new(
                ErrorKind::UnterminatedString,
                "String starting with quote must end with quote",
            ).with_location(line_number, 1));
        }
        let inner = &trimmed[1..trimmed.len() - 1];
        let unescaped = unescape_string(inner, line_number)?;
        return Ok(Value::String(unescaped));
    }

    match trimmed {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        "null" => return Ok(Value::Null),
        _ => {}
    }

    if let Ok(num) = parse_number(trimmed) {
        return Ok(Value::Number(num));
    }

    Ok(Value::String(trimmed.to_string()))
}

fn parse_number(s: &str) -> Result<Number> {
    if s.starts_with('0') && s.len() > 1 && s.chars().nth(1).unwrap().is_ascii_digit() {
        return Err(Error::custom("Leading zeros not allowed"));
    }

    if s.contains('.') || s.contains('e') || s.contains('E') {
        let f = s.parse::<f64>().map_err(|_| Error::custom("Invalid number"))?;
        Ok(Number::F64(f))
    } else if s.starts_with('-') {
        let i = s.parse::<i64>().map_err(|_| Error::custom("Invalid number"))?;
        Ok(Number::I64(i))
    } else {
        match s.parse::<u64>() {
            Ok(u) => Ok(Number::U64(u)),
            Err(_) => {
                let i = s.parse::<i64>().map_err(|_| Error::custom("Invalid number"))?;
                Ok(Number::I64(i))
            }
        }
    }
}

fn unescape_string(s: &str, line_number: usize) -> Result<String> {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some(other) => {
                    return Err(Error::new(
                        ErrorKind::InvalidEscape,
                        format!("Invalid escape sequence: \\{}", other),
                    ).with_location(line_number, 1));
                }
                None => {
                    return Err(Error::new(
                        ErrorKind::UnterminatedString,
                        "Backslash at end of string",
                    ).with_location(line_number, 1));
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn expand_paths(value: Value, options: &DecoderOptions) -> Result<Value> {
    match value {
        Value::Object(obj) => {
            let mut result = Map::new();

            for (key, val) in obj {
                let expanded_val = expand_paths(val, options)?;

                // Check if key was originally quoted (marked with \x00 prefix)
                let (is_quoted, clean_key) = if key.starts_with('\x00') {
                    (true, key[1..].to_string())
                } else {
                    (false, key.clone())
                };

                if !is_quoted && options.expand_paths == PathExpansion::Safe && clean_key.contains('.') {
                    let segments: Vec<&str> = clean_key.split('.').collect();
                    let all_safe = segments.iter().all(|seg| {
                        !seg.is_empty() &&
                        seg.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) &&
                        seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    });

                    if all_safe && segments.len() > 1 {
                        merge_path(&mut result, &segments, expanded_val, options)?;
                        continue;
                    }
                }

                let final_key = if is_quoted { clean_key } else { key };
                if let Some(existing) = result.get(&final_key) {
                    if options.strict {
                        if !matches!(existing, Value::Object(_)) || !matches!(&expanded_val, Value::Object(_)) {
                            return Err(Error::new(
                                ErrorKind::ExpansionConflict,
                                format!("Path expansion conflict at '{}'", final_key),
                            ));
                        }
                    }
                }
                result.insert(final_key, expanded_val);
            }

            Ok(Value::Object(result))
        }
        Value::Array(arr) => {
            Ok(Value::Array(
                arr.into_iter()
                    .map(|v| expand_paths(v, options))
                    .collect::<Result<Vec<_>>>()?
            ))
        }
        other => Ok(other),
    }
}

fn merge_path(
    obj: &mut Map<String, Value>,
    segments: &[&str],
    value: Value,
    options: &DecoderOptions,
) -> Result<()> {
    if segments.is_empty() {
        return Ok(());
    }

    if segments.len() == 1 {
        let key = segments[0].to_string();
        if let Some(existing) = obj.get(&key) {
            if !matches!(existing, Value::Object(_)) || !matches!(&value, Value::Object(_)) {
                if options.strict {
                    return Err(Error::new(
                        ErrorKind::ExpansionConflict,
                        format!("Path expansion conflict at '{}'", key),
                    ));
                }
            }
        }
        obj.insert(key, value);
        return Ok(());
    }

    let first = segments[0].to_string();
    let rest = &segments[1..];

    match obj.get_mut(&first) {
        Some(Value::Object(nested)) => {
            merge_path(nested, rest, value, options)?;
        }
        Some(_) => {
            if options.strict {
                return Err(Error::new(
                    ErrorKind::ExpansionConflict,
                    format!("Path expansion conflict at '{}'", first),
                ));
            }
            let mut nested = Map::new();
            merge_path(&mut nested, rest, value, options)?;
            obj.insert(first, Value::Object(nested));
        }
        None => {
            let mut nested = Map::new();
            merge_path(&mut nested, rest, value, options)?;
            obj.insert(first, Value::Object(nested));
        }
    }

    Ok(())
}

impl<'de> de::Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else {
                    visitor.visit_f64(n.as_f64())
                }
            }
            Value::String(s) => visitor.visit_string(s),
            Value::Array(arr) => visitor.visit_seq(SeqDeserializer::new(arr)),
            Value::Object(obj) => visitor.visit_map(MapDeserializer::new(obj)),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Value>,
}

impl SeqDeserializer {
    fn new(values: Vec<Value>) -> Self {
        SeqDeserializer {
            iter: values.into_iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }
}

struct MapDeserializer {
    iter: indexmap::map::IntoIter<String, Value>,
    value: Option<Value>,
}

impl MapDeserializer {
    fn new(map: Map<String, Value>) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(StringDeserializer(key)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(Error::custom("Value is missing")),
        }
    }
}

struct StringDeserializer(String);

impl<'de> de::Deserializer<'de> for StringDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_string(self.0)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
