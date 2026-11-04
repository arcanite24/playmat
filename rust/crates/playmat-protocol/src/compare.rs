//! Comparing Java's and Rust's answers in `shadow` routing. Views carry ids made fresh for every answer (a card
//! view's `cardId`), so two UUIDs count as the same value; anything else must match exactly.

use serde_json::Value;

/// None when the answers match, else where they first differ (a JSON pointer) and how
pub fn difference(java: &Value, rust: &Value) -> Option<String> {
    let mut path = String::new();
    walk(java, rust, &mut path)
}

fn walk(java: &Value, rust: &Value, path: &mut String) -> Option<String> {
    match (java, rust) {
        (Value::String(a), Value::String(b)) if a == b || (is_uuid(a) && is_uuid(b)) => None,
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: {} items in Java, {} in Rust", a.len(), b.len()));
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                let len = path.len();
                path.push_str(&format!("/{i}"));
                if let Some(found) = walk(x, y, path) {
                    return Some(found);
                }
                path.truncate(len);
            }
            None
        }
        (Value::Object(a), Value::Object(b)) => {
            if let Some(key) = a.keys().find(|k| !b.contains_key(*k)) {
                return Some(format!("{path}/{key}: only in Java"));
            }
            if let Some(key) = b.keys().find(|k| !a.contains_key(*k)) {
                return Some(format!("{path}/{key}: only in Rust"));
            }
            for (key, x) in a {
                let len = path.len();
                path.push('/');
                path.push_str(key);
                if let Some(found) = walk(x, &b[key], path) {
                    return Some(found);
                }
                path.truncate(len);
            }
            None
        }
        (a, b) if a == b => None,
        (a, b) => Some(format!("{path}: Java {}, Rust {}", short(a), short(b))),
    }
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.len() > 120 {
        format!("{}...", &text[..text.char_indices().nth(117).map(|(i, _)| i).unwrap_or(text.len())])
    } else {
        text
    }
}

/// 8-4-4-4-12 hex digits
pub fn is_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ids_may_differ_values_may_not() {
        let a = json!([{"cardId": "0b6f2c1e-3d4a-4e5f-8a9b-1c2d3e4f5a6b", "name": "Shock", "cmc": 1}]);
        let b = json!([{"cardId": "9f8e7d6c-5b4a-4321-8fed-cba987654321", "name": "Shock", "cmc": 1}]);
        assert_eq!(difference(&a, &b), None);
        let c = json!([{"cardId": "9f8e7d6c-5b4a-4321-8fed-cba987654321", "name": "Shock", "cmc": 2}]);
        assert_eq!(difference(&a, &c).unwrap(), "/0/cmc: Java 1, Rust 2");
        assert_eq!(difference(&json!({"a": 1}), &json!({})).unwrap(), "/a: only in Java");
        assert_eq!(difference(&json!([1]), &json!([1, 2])).unwrap(), ": 1 items in Java, 2 in Rust");
        assert!(difference(&json!("x"), &json!("0b6f2c1e-3d4a-4e5f-8a9b-1c2d3e4f5a6b")).is_some());
    }
}
