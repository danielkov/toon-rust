use serde::Serialize;
use serde_toon2::to_string;

#[derive(Serialize)]
struct SimpleStruct {
    id: i64,
    name: String,
    active: bool,
}

#[test]
fn test_simple_object() {
    let obj = SimpleStruct {
        id: 123,
        name: "Ada".to_string(),
        active: true,
    };

    let result = to_string(&obj).unwrap();
    assert_eq!(result, "id: 123\nname: Ada\nactive: true");
}

#[derive(Serialize)]
struct ArrayStruct {
    tags: Vec<String>,
}

#[test]
fn test_primitive_array() {
    let obj = ArrayStruct {
        tags: vec!["reading".to_string(), "gaming".to_string()],
    };

    let result = to_string(&obj).unwrap();
    assert_eq!(result, "tags[2]: reading,gaming");
}

#[test]
fn test_empty_object() {
    use std::collections::HashMap;
    let map: HashMap<String, String> = HashMap::new();
    let result = to_string(&map).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_root_primitive() {
    let result = to_string(&"hello").unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_root_number() {
    let result = to_string(&42).unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_root_bool() {
    let result = to_string(&true).unwrap();
    assert_eq!(result, "true");
}
