use serde::Serialize;
use serde_toon2::to_string;

#[derive(Serialize)]
struct NestedObject {
    user: User,
}

#[derive(Serialize)]
struct User {
    id: i64,
    name: String,
}

#[test]
fn test_nested_object() {
    let obj = NestedObject {
        user: User {
            id: 123,
            name: "Ada".to_string(),
        },
    };

    let result = to_string(&obj).unwrap();
    let expected = "user:\n  id: 123\n  name: Ada";
    assert_eq!(result, expected);
}

#[derive(Serialize)]
struct TabularArrayStruct {
    items: Vec<Item>,
}

#[derive(Serialize)]
struct Item {
    sku: String,
    qty: i64,
    price: f64,
}

#[test]
fn test_tabular_array() {
    let obj = TabularArrayStruct {
        items: vec![
            Item {
                sku: "A1".to_string(),
                qty: 2,
                price: 9.99,
            },
            Item {
                sku: "B2".to_string(),
                qty: 1,
                price: 14.5,
            },
        ],
    };

    let result = to_string(&obj).unwrap();
    let expected = "items[2]{sku,qty,price}:\n  A1,2,9.99\n  B2,1,14.5";
    assert_eq!(result, expected);
}

#[test]
fn test_root_array() {
    let arr = vec!["x", "y", "true"];
    let result = to_string(&arr).unwrap();
    let expected = "[3]: x,y,\"true\"";
    assert_eq!(result, expected);
}

#[derive(Serialize)]
struct ArrayOfArraysStruct {
    pairs: Vec<Vec<String>>,
}

#[test]
fn test_array_of_arrays() {
    let obj = ArrayOfArraysStruct {
        pairs: vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ],
    };

    let result = to_string(&obj).unwrap();
    let expected = "pairs[2]:\n  - [2]: a,b\n  - [2]: c,d";
    assert_eq!(result, expected);
}
