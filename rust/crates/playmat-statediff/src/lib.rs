//! The difference between two JSON trees, so a game update can be sent as a small patch. A port of the Java
//! bridge's `mage.server.websocket.StateDiff`; both must produce the same patches, byte for byte
//! (`tests/fixture.rs` checks it against the fixture the Java and TypeScript tests share).
//!
//! A patch is an operation, one of:
//! - `[value]`: the value is replaced by `value` (any JSON, null included)
//! - `[]`: the object key is removed
//! - `{"key": operation, ...}`: an object changes key by key; keys without an operation keep their value, new keys
//!   are added after the existing ones
//! - `["o", {"key": operation or 0, ...}]`: an object gets exactly the listed keys, in that order; 0 keeps the old
//!   value
//! - `[length, {"index": operation, ...}]`: an array changes element by element, then is cut or grown to `length`
//!
//! Needs serde_json's `preserve_order` (key order is part of the format) and `arbitrary_precision` (numbers compare
//! and print as the text they were written as, like Gson's).

use serde_json::{Map, Value};

const ORDERED: &str = "o";

/// The operation that turns `from` into `to`, or None when they are equal.
pub fn diff(from: &Value, to: &Value) -> Option<Value> {
    match (from, to) {
        (Value::Object(a), Value::Object(b)) => diff_objects(a, b),
        (Value::Array(a), Value::Array(b)) => diff_arrays(a, b),
        (Value::Object(_) | Value::Array(_), _) | (_, Value::Object(_) | Value::Array(_)) => Some(set(to)),
        _ => {
            if same_primitive(from, to) {
                None
            } else {
                Some(set(to))
            }
        }
    }
}

fn same_primitive(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x.to_string() == y.to_string(),
        _ => false,
    }
}

fn set(value: &Value) -> Value {
    Value::Array(vec![value.clone()])
}

fn diff_objects(from: &Map<String, Value>, to: &Map<String, Value>) -> Option<Value> {
    if let Some(patch) = diff_same_keys(from, to) {
        return if patch.is_empty() { None } else { Some(Value::Object(patch)) };
    }
    if !keeps_key_order(from, to) {
        return Some(diff_reordered(from, to));
    }
    let mut patch = Map::new();
    for key in from.keys() {
        if !to.contains_key(key) {
            patch.insert(key.clone(), Value::Array(Vec::new()));
        }
    }
    for (key, value) in to {
        let operation = match from.get(key) {
            None => Some(set(value)),
            Some(old) => diff(old, value),
        };
        if let Some(operation) = operation {
            patch.insert(key.clone(), operation);
        }
    }
    if patch.is_empty() {
        None
    } else {
        Some(Value::Object(patch))
    }
}

/// The common case, the same keys in the same order. None when the keys differ.
fn diff_same_keys(from: &Map<String, Value>, to: &Map<String, Value>) -> Option<Map<String, Value>> {
    if from.len() != to.len() || !from.keys().eq(to.keys()) {
        return None;
    }
    let mut patch = Map::new();
    for ((_, old), (key, value)) in from.iter().zip(to.iter()) {
        if let Some(operation) = diff(old, value) {
            patch.insert(key.clone(), operation);
        }
    }
    Some(patch)
}

fn diff_reordered(from: &Map<String, Value>, to: &Map<String, Value>) -> Value {
    let mut keys = Map::new();
    for (key, value) in to {
        let operation = match from.get(key) {
            None => Some(set(value)),
            Some(old) => diff(old, value),
        };
        keys.insert(key.clone(), operation.unwrap_or_else(|| Value::from(0)));
    }
    Value::Array(vec![Value::from(ORDERED), Value::Object(keys)])
}

/// A patch keeps the remaining keys in place and adds new ones at the end: true when that gives `to`'s order.
fn keeps_key_order(from: &Map<String, Value>, to: &Map<String, Value>) -> bool {
    let mut kept = from.keys().filter(|key| to.contains_key(*key));
    let mut adding = false;
    for key in to.keys() {
        if from.contains_key(key) {
            if adding || kept.next() != Some(key) {
                return false;
            }
        } else {
            adding = true;
        }
    }
    true
}

fn diff_arrays(from: &[Value], to: &[Value]) -> Option<Value> {
    let mut changes = Map::new();
    let common = from.len().min(to.len());
    let mut reused = false; // some old element is kept or patched
    for i in 0..common {
        match diff(&from[i], &to[i]) {
            None => reused = true,
            Some(operation) => {
                reused |= !is_set(&operation);
                changes.insert(i.to_string(), operation);
            }
        }
    }
    if changes.is_empty() && from.len() == to.len() {
        return None;
    }
    if !reused {
        return Some(set(&Value::Array(to.to_vec())));
    }
    for (i, value) in to.iter().enumerate().skip(common) {
        changes.insert(i.to_string(), set(value));
    }
    Some(Value::Array(vec![Value::from(to.len()), Value::Object(changes)]))
}

fn is_set(operation: &Value) -> bool {
    matches!(operation, Value::Array(a) if a.len() == 1)
}

fn is_remove(operation: &Value) -> bool {
    matches!(operation, Value::Array(a) if a.is_empty())
}

/// A patch that does not fit the tree it is applied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError(pub String);

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PatchError {}

fn describe(value: Option<&Value>) -> &'static str {
    match value {
        None => "nothing",
        Some(Value::Object(_)) => "an object",
        Some(Value::Array(_)) => "an array",
        Some(_) => "a value",
    }
}

/// Applies a patch made by [`diff`] to `from`. `operation` None means no change.
pub fn apply(from: Option<&Value>, operation: Option<&Value>) -> Result<Value, PatchError> {
    let Some(operation) = operation else {
        return from.cloned().ok_or_else(|| PatchError("No change to nothing".into()));
    };
    match operation {
        Value::Object(patch) => {
            let Some(Value::Object(old)) = from else {
                return Err(PatchError(format!("Object patch on {}", describe(from))));
            };
            let mut result = Map::new();
            for (key, value) in old {
                match patch.get(key) {
                    None => {
                        result.insert(key.clone(), value.clone());
                    }
                    Some(op) if is_remove(op) => {}
                    Some(op) => {
                        result.insert(key.clone(), apply(Some(value), Some(op))?);
                    }
                }
            }
            for (key, op) in patch {
                if !old.contains_key(key) {
                    if is_remove(op) {
                        return Err(PatchError(format!("Removes missing key {key}")));
                    }
                    result.insert(key.clone(), apply(None, Some(op))?);
                }
            }
            Ok(Value::Object(result))
        }
        Value::Array(array) if array.len() == 1 => Ok(array[0].clone()),
        Value::Array(array) if array.len() == 2 && array[0].is_string() => {
            let (Some(Value::Object(old)), Some(Value::Object(keys))) = (from, array.get(1)) else {
                return Err(PatchError(format!("Bad operation on {}", describe(from))));
            };
            if array[0].as_str() != Some(ORDERED) {
                return Err(PatchError("Bad ordered operation".into()));
            }
            let mut result = Map::new();
            for (key, op) in keys {
                let previous = old.get(key);
                let keep = !op.is_array() && !op.is_object();
                let value = if keep {
                    previous.cloned().ok_or_else(|| PatchError(format!("Keeps missing key {key}")))?
                } else {
                    apply(previous, Some(op))?
                };
                result.insert(key.clone(), value);
            }
            Ok(Value::Object(result))
        }
        Value::Array(array) if array.len() == 2 => {
            let (Some(Value::Array(old)), Some(length), Some(Value::Object(changes))) =
                (from, array[0].as_u64(), array.get(1))
            else {
                return Err(PatchError(format!("Bad operation on {}", describe(from))));
            };
            let mut result = Vec::with_capacity(length as usize);
            for i in 0..length as usize {
                let change = changes.get(&i.to_string());
                if i < old.len() {
                    result.push(match change {
                        None => old[i].clone(),
                        Some(op) => apply(Some(&old[i]), Some(op))?,
                    });
                } else if let Some(op) = change {
                    result.push(apply(None, Some(op))?);
                } else {
                    return Err(PatchError(format!("Missing new element {i}")));
                }
            }
            Ok(Value::Array(result))
        }
        _ => Err(PatchError(format!("Bad operation on {}", describe(from)))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn round_trip(from: Value, to: Value) -> Option<Value> {
        let patch = diff(&from, &to);
        let applied = apply(Some(&from), patch.as_ref()).unwrap();
        assert_eq!(serde_json::to_string(&applied).unwrap(), serde_json::to_string(&to).unwrap());
        patch
    }

    #[test]
    fn equal_trees_give_no_patch() {
        assert_eq!(round_trip(json!({"a": 1, "b": [1, 2]}), json!({"a": 1, "b": [1, 2]})), None);
    }

    #[test]
    fn changed_key() {
        assert_eq!(round_trip(json!({"a": 1, "b": 2}), json!({"a": 1, "b": 3})), Some(json!({"b": [3]})));
    }

    #[test]
    fn removed_and_added_keys() {
        assert_eq!(round_trip(json!({"a": 1, "b": 2}), json!({"a": 1, "c": null})), Some(json!({"b": [], "c": [null]})));
    }

    #[test]
    fn reordered_keys() {
        assert_eq!(round_trip(json!({"a": 1, "b": 2}), json!({"b": 2, "a": 5})), Some(json!(["o", {"b": 0, "a": [5]}])));
    }

    #[test]
    fn arrays() {
        assert_eq!(round_trip(json!([1, 2, 3]), json!([1, 5])), Some(json!([2, {"1": [5]}])));
        assert_eq!(round_trip(json!([1, 2]), json!([1, 2, 3])), Some(json!([3, {"2": [3]}])));
        assert_eq!(round_trip(json!([1, 2]), json!([3, 4])), Some(json!([[3, 4]])));
    }

    #[test]
    fn numbers_compare_as_written() {
        let from: Value = serde_json::from_str("{\"a\": 1.0}").unwrap();
        let to: Value = serde_json::from_str("{\"a\": 1}").unwrap();
        assert_eq!(serde_json::to_string(&diff(&from, &to).unwrap()).unwrap(), "{\"a\":[1]}");
    }

    #[test]
    fn bad_patches_are_refused() {
        assert!(apply(Some(&json!([1])), Some(&json!({"a": [1]}))).is_err());
        assert!(apply(Some(&json!({"a": 1})), Some(&json!({"b": []}))).is_err());
    }
}
