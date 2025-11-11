use serde_toon2::{from_str, to_string, Value};
use serde_json;

#[test]
fn test_json_to_toon_to_json() {
    let json_input = r#"{
  "users": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ],
  "count": 2
}"#;

    let value: serde_json::Value = serde_json::from_str(json_input).unwrap();

    let toon_value: Value = serde_json::from_value(value.clone()).unwrap();
    let toon_str = to_string(&toon_value).unwrap();

    let parsed_toon: Value = from_str(&toon_str).unwrap();
    let json_output = serde_json::to_value(&parsed_toon).unwrap();

    assert_eq!(value, json_output);
}

#[test]
fn test_toon_to_json_to_toon() {
    let toon_input = r#"count: 2
users[2]{id,name,email}:
  1,Alice,alice@example.com
  2,Bob,bob@example.com"#;

    let toon_value: Value = from_str(toon_input).unwrap();

    let json_value = serde_json::to_value(&toon_value).unwrap();
    let json_str = serde_json::to_string(&json_value).unwrap();

    let parsed_json: Value = serde_json::from_str(&json_str).unwrap();
    let toon_output = to_string(&parsed_json).unwrap();

    let final_toon: Value = from_str(&toon_output).unwrap();

    assert_eq!(toon_value, final_toon);
}

#[test]
fn test_complex_nested_reversibility() {
    let json_input = r#"{
  "data": {
    "items": [
      {"id": 1, "value": "a"},
      {"id": 2, "value": "b"}
    ],
    "metadata": {
      "version": "1.0",
      "author": "test"
    }
  }
}"#;

    let value: serde_json::Value = serde_json::from_str(json_input).unwrap();
    let toon_value: Value = serde_json::from_value(value.clone()).unwrap();
    let toon_str = to_string(&toon_value).unwrap();

    let parsed_toon: Value = from_str(&toon_str).unwrap();
    let json_output = serde_json::to_value(&parsed_toon).unwrap();

    assert_eq!(value, json_output);
}

#[test]
fn test_primitives_reversibility() {
    let test_cases = vec![
        (r#"42"#, "42"),
        (r#""hello""#, "hello"),
        (r#"true"#, "true"),
        (r#"false"#, "false"),
        (r#"null"#, "null"),
        (r#"3.14"#, "3.14"),
        (r#"-42"#, "-42"),
    ];

    for (json_input, expected_toon) in test_cases {
        let value: serde_json::Value = serde_json::from_str(json_input).unwrap();
        let toon_value: Value = serde_json::from_value(value.clone()).unwrap();
        let toon_str = to_string(&toon_value).unwrap();

        assert_eq!(toon_str, expected_toon);

        let parsed_toon: Value = from_str(&toon_str).unwrap();
        let json_output = serde_json::to_value(&parsed_toon).unwrap();

        assert_eq!(value, json_output);
    }
}

#[test]
fn test_array_reversibility() {
    let json_input = r#"[1, 2, 3, 4, 5]"#;

    let value: serde_json::Value = serde_json::from_str(json_input).unwrap();
    let toon_value: Value = serde_json::from_value(value.clone()).unwrap();
    let toon_str = to_string(&toon_value).unwrap();

    assert_eq!(toon_str, "[5]: 1,2,3,4,5");

    let parsed_toon: Value = from_str(&toon_str).unwrap();
    let json_output = serde_json::to_value(&parsed_toon).unwrap();

    assert_eq!(value, json_output);
}

#[test]
fn test_empty_structures_reversibility() {
    let test_cases = vec![
        (r#"{}"#, ""),
        (r#"[]"#, "[0]:"),
        (r#"{"empty": {}}"#, "empty:"),
        (r#"{"empty": []}"#, "empty[0]:"),
    ];

    for (json_input, expected_toon) in test_cases {
        let value: serde_json::Value = serde_json::from_str(json_input).unwrap();
        let toon_value: Value = serde_json::from_value(value.clone()).unwrap();
        let toon_str = to_string(&toon_value).unwrap();

        assert_eq!(toon_str, expected_toon, "Failed for input: {}", json_input);

        let parsed_toon: Value = from_str(&toon_str).unwrap();
        let json_output = serde_json::to_value(&parsed_toon).unwrap();

        assert_eq!(value, json_output, "Reversibility failed for: {}", json_input);
    }
}

#[test]
fn test_special_characters_reversibility() {
    let json_input = r#"{
  "message": "Hello\nWorld",
  "path": "C:\\Users\\test",
  "quote": "say \"hello\""
}"#;

    let value: serde_json::Value = serde_json::from_str(json_input).unwrap();
    let toon_value: Value = serde_json::from_value(value.clone()).unwrap();
    let toon_str = to_string(&toon_value).unwrap();

    let parsed_toon: Value = from_str(&toon_str).unwrap();
    let json_output = serde_json::to_value(&parsed_toon).unwrap();

    assert_eq!(value, json_output);
}

#[test]
fn test_numbers_canonical_format() {
    let test_cases = vec![
        (1.0, "1"),
        (1.5, "1.5"),
        (-0.0, "0"),
        (42.0, "42"),
        (3.14159, "3.14159"),
    ];

    for (num, expected) in test_cases {
        let json_input = serde_json::json!(num);
        let toon_value: Value = serde_json::from_value(json_input.clone()).unwrap();
        let toon_str = to_string(&toon_value).unwrap();

        assert_eq!(toon_str, expected);

        let parsed_toon: Value = from_str(&toon_str).unwrap();
        let json_output = serde_json::to_value(&parsed_toon).unwrap();

        // Compare as floats for numeric equality
        match (json_input.as_f64(), json_output.as_f64()) {
            (Some(a), Some(b)) => {
                assert!((a - b).abs() < f64::EPSILON, "Numbers not equal: {} vs {}", a, b);
            }
            _ => panic!("Expected numbers"),
        }
    }
}
