use serde::Deserialize;
use serde_toon2::{Value, from_str};

#[test]
fn test_primitive_string() {
    let result: Value = from_str("hello").unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

#[test]
fn test_primitive_number() {
    let result: Value = from_str("42").unwrap();
    assert_eq!(result.as_i64().unwrap(), 42);
}

#[test]
fn test_primitive_bool() {
    let result: Value = from_str("true").unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn test_primitive_null() {
    let result: Value = from_str("null").unwrap();
    assert!(result.is_null());
}

#[test]
fn test_quoted_string() {
    let result: Value = from_str("\"hello world\"").unwrap();
    assert_eq!(result.as_str().unwrap(), "hello world");
}

#[test]
fn test_escaped_string() {
    let result: Value = from_str(r#""line1\nline2""#).unwrap();
    assert_eq!(result.as_str().unwrap(), "line1\nline2");
}

#[test]
fn test_simple_object() {
    let input = "id: 123\nname: Ada\nactive: true";
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("id").unwrap().as_i64().unwrap(), 123);
    assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "Ada");
    assert!(obj.get("active").unwrap().as_bool().unwrap());
}

#[test]
fn test_empty_object() {
    let result: Value = from_str("").unwrap();
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn test_nested_object() {
    let input = "user:\n  id: 123\n  name: Ada";
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    let user = obj.get("user").unwrap().as_object().unwrap();
    assert_eq!(user.get("id").unwrap().as_i64().unwrap(), 123);
    assert_eq!(user.get("name").unwrap().as_str().unwrap(), "Ada");
}

#[test]
fn test_inline_array() {
    let input = "tags[3]: reading,gaming,coding";
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    let tags = obj.get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0].as_str().unwrap(), "reading");
    assert_eq!(tags[1].as_str().unwrap(), "gaming");
    assert_eq!(tags[2].as_str().unwrap(), "coding");
}

#[test]
fn test_empty_array() {
    let input = "items[0]:";
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    let items = obj.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 0);
}

#[test]
fn test_tabular_array() {
    let input = "items[2]{id,name}:\n  1,Alice\n  2,Bob";
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    let items = obj.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2);

    let item1 = items[0].as_object().unwrap();
    assert_eq!(item1.get("id").unwrap().as_i64().unwrap(), 1);
    assert_eq!(item1.get("name").unwrap().as_str().unwrap(), "Alice");

    let item2 = items[1].as_object().unwrap();
    assert_eq!(item2.get("id").unwrap().as_i64().unwrap(), 2);
    assert_eq!(item2.get("name").unwrap().as_str().unwrap(), "Bob");
}

#[derive(Debug, Deserialize, PartialEq)]
struct Person {
    id: i64,
    name: String,
    active: bool,
}

#[test]
fn test_deserialize_to_struct() {
    let input = "id: 123\nname: Ada\nactive: true";
    let result: Person = from_str(input).unwrap();

    assert_eq!(result.id, 123);
    assert_eq!(result.name, "Ada");
    assert!(result.active);
}

#[test]
fn test_quoted_key() {
    let input = r#""my-key": 123"#;
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("my-key").unwrap().as_i64().unwrap(), 123);
}

#[test]
fn test_quoted_value_with_delimiter() {
    let input = r#"note: "a,b,c""#;
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("note").unwrap().as_str().unwrap(), "a,b,c");
}

#[test]
fn test_quoted_primitives_stay_strings() {
    let input = r#"v1: "true"
v2: "42"
v3: "null""#;
    let result: Value = from_str(input).unwrap();

    let obj = result.as_object().unwrap();
    assert_eq!(obj.get("v1").unwrap().as_str().unwrap(), "true");
    assert_eq!(obj.get("v2").unwrap().as_str().unwrap(), "42");
    assert_eq!(obj.get("v3").unwrap().as_str().unwrap(), "null");
}
