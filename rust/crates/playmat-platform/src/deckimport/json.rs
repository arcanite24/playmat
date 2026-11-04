//! `SourceSupport`'s forgiving JSON readers, with Gson's `JsonPrimitive` conversions.

use super::{Failure, Reason};
use serde_json::{Map, Value};

pub type Object = Map<String, Value>;

pub fn parse_object(body: &str, site: &str) -> Result<Object, Failure> {
    // Gson reads an empty body as JsonNull: not an object
    if body.trim_matches(|c: char| c.is_ascii_whitespace()).is_empty() {
        return Err(Failure::new(Reason::SiteChanged, format!("{site} answered with something other than a deck")));
    }
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) => Err(Failure::new(Reason::SiteChanged, format!("{site} answered with something other than a deck"))),
        Err(_) => Err(Failure::new(Reason::SiteChanged, format!("{site} answered with something other than JSON"))),
    }
}

pub fn object<'a>(parent: Option<&'a Object>, key: &str) -> Option<&'a Object> {
    parent?.get(key)?.as_object()
}

pub fn array<'a>(parent: Option<&'a Object>, key: &str) -> &'a [Value] {
    parent.and_then(|p| p.get(key)).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// `JsonPrimitive.getAsString()`, or None when the value isn't a primitive
pub fn primitive_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

pub fn string(parent: Option<&Object>, key: &str) -> Option<String> {
    primitive_string(parent?.get(key)?)
}

/// `JsonPrimitive.getAsInt()` with the fallback for values that aren't numbers
pub fn integer(parent: Option<&Object>, key: &str, fallback: i32) -> i32 {
    match parent.and_then(|p| p.get(key)) {
        Some(Value::Number(number)) => number_long(&number.to_string()).map(|n| n as i32).unwrap_or(fallback),
        Some(Value::String(text)) => text.parse::<i32>().unwrap_or(fallback),
        _ => fallback,
    }
}

pub fn long_value(parent: Option<&Object>, key: &str, fallback: i64) -> i64 {
    match parent.and_then(|p| p.get(key)) {
        Some(Value::Number(number)) => number_long(&number.to_string()).unwrap_or(fallback),
        Some(Value::String(text)) => text.parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    }
}

/// `LazilyParsedNumber.longValue()`: the integer, else the decimal's whole part
fn number_long(text: &str) -> Option<i64> {
    if let Ok(whole) = text.parse::<i64>() {
        return Some(whole);
    }
    let value = text.parse::<f64>().ok()?;
    value.is_finite().then_some(value.trunc() as i64)
}

/// `JsonPrimitive.getAsBoolean()`, false for anything that isn't a primitive
pub fn boolean(parent: Option<&Object>, key: &str) -> bool {
    match parent.and_then(|p| p.get(key)) {
        Some(Value::Bool(flag)) => *flag,
        Some(other) => primitive_string(other).is_some_and(|t| t.eq_ignore_ascii_case("true")),
        None => false,
    }
}

/// `Instant.parse` ("2026-04-27T01:04:41.158476Z", or with an offset) as epoch milliseconds
pub fn iso_millis(value: Option<&str>) -> Option<i64> {
    let text = value?.to_ascii_uppercase();
    let b = text.as_bytes();
    let digits = |from: usize, len: usize| -> Option<i64> {
        let part = text.get(from..from + len)?;
        part.bytes().all(|c| c.is_ascii_digit()).then(|| part.parse().ok())?
    };
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let (year, month, day) = (digits(0, 4)?, digits(5, 2)?, digits(8, 2)?);
    let (hour, minute, second) = (digits(11, 2)?, digits(14, 2)?, digits(17, 2)?);
    let mut at = 19;
    let mut nanos = 0i64;
    if b.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while at < b.len() && b[at].is_ascii_digit() {
            at += 1;
        }
        let fraction = &text[start..at];
        if fraction.len() > 9 {
            return None;
        }
        nanos = format!("{fraction:0<9}").parse().ok()?;
    }
    let offset_seconds = match &text[at..] {
        "Z" => 0,
        offset if (offset.len() == 6 || offset.len() == 9) && (offset.starts_with('+') || offset.starts_with('-')) => {
            let ob = offset.as_bytes();
            if ob[3] != b':' || (offset.len() == 9 && ob[6] != b':') {
                return None;
            }
            let h = digits(at + 1, 2)?;
            let m = digits(at + 4, 2)?;
            let s = if offset.len() == 9 { digits(at + 7, 2)? } else { 0 };
            if h > 18 || m > 59 || s > 59 {
                return None;
            }
            let total = h * 3600 + m * 60 + s;
            if offset.starts_with('-') {
                -total
            } else {
                total
            }
        }
        _ => return None,
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=12).contains(&month) || day < 1 || day > month_days[(month - 1) as usize] {
        return None;
    }
    // ISO_INSTANT takes 24:00 and a leap second (23:59:60, read as :59)
    let (hour, second, next_day) = match (hour, minute, second) {
        (24, 0, 0) if nanos == 0 => (0, 0, 1),
        (23, 59, 60) => (23, 59, 0),
        (h, m, s) if h < 24 && m < 60 && s < 60 => (h, s, 0),
        _ => return None,
    };
    let days = days_from_civil(year, month, day) + next_day;
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_seconds;
    Some(seconds * 1000 + nanos / 1_000_000)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn instants() {
        assert_eq!(iso_millis(Some("1970-01-01T00:00:00Z")), Some(0));
        assert_eq!(iso_millis(Some("2026-04-27T01:04:41.158476Z")), Some(1_777_251_881_158));
        assert_eq!(iso_millis(Some("2026-04-27T03:04:41+02:00")), Some(1_777_251_881_000));
        assert_eq!(iso_millis(Some("2024-02-29t00:00:00z")), Some(1_709_164_800_000));
        assert_eq!(iso_millis(Some("2023-02-29T00:00:00Z")), None);
        assert_eq!(iso_millis(Some("2026-04-27 01:04:41Z")), None);
        assert_eq!(iso_millis(Some("2026-04-27T01:04Z")), None);
        assert_eq!(iso_millis(Some("")), None);
    }

    #[test]
    fn primitives() {
        let object = json!({"n": 3.7, "s": "12", "bad": "1.5", "b": "TRUE", "big": 1786314484436i64, "null": null});
        let object = object.as_object();
        assert_eq!(integer(object, "n", -1), 3);
        assert_eq!(integer(object, "s", -1), 12);
        assert_eq!(integer(object, "bad", -1), -1);
        assert_eq!(integer(object, "null", -1), -1);
        assert_eq!(long_value(object, "big", -1), 1_786_314_484_436);
        assert!(boolean(object, "b"));
        assert_eq!(string(object, "n").as_deref(), Some("3.7"));
        assert_eq!(string(object, "null"), None);
    }
}
