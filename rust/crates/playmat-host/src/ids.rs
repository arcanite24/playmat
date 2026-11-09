//! The client's ids. The engine's views name objects by decision-log reference (`H0.3`, `B12`, `S0`, `B4a1`); the
//! client needs ids that stay the same while an object moves (Java's card ids). Each object gets a UUID made from the
//! game's id and the engine's object id, and every reference in a rendered view or prompt is replaced by its UUID.
//! The UUIDs of the last render map back to references, which the engine resolves into answers.

use playmat_engine::replay::resolve;
use playmat_engine::state::{Game, ObjId};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Fields whose text is never a reference (a set code like `E01` would read as one)
const NOT_IDS: &[&str] = &[
    "expansionSetCode",
    "cardNumber",
    "setCode",
    "tokenSetCode",
    "tokenDescriptor",
    "imageFileName",
    "name",
    "displayName",
    "displayFullName",
    "message",
    "phase",
    "step",
];

pub struct Ids {
    salt: u64,
    /// each seat's player id, as Java gave it
    pub players: Vec<String>,
    /// the references of the last render, by UUID
    refs: HashMap<String, String>,
}

impl Ids {
    pub fn new(game_id: &str, players: Vec<String>) -> Ids {
        let mut h = DefaultHasher::new();
        game_id.hash(&mut h);
        Ids { salt: h.finish(), players, refs: HashMap::new() }
    }

    fn uuid(&self, parts: impl Hash) -> String {
        let mut a = DefaultHasher::new();
        (self.salt, 1u8, &parts).hash(&mut a);
        let mut b = DefaultHasher::new();
        (self.salt, 2u8, &parts).hash(&mut b);
        let (a, b) = (a.finish(), b.finish());
        // a version 4 UUID's layout, so it parses as one
        format!(
            "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
            (a >> 32) as u32,
            (a >> 16) as u16,
            a as u16 & 0x0fff,
            (b >> 48) as u16 & 0x3fff | 0x8000,
            b & 0xffff_ffff_ffff
        )
    }

    /// An object's id: the same in every zone
    pub fn object(&self, id: ObjId) -> String {
        self.uuid(("obj", id.0))
    }

    /// The game's chat id (the game log's lines are chat messages)
    pub fn chat(&self) -> String {
        self.uuid("chat")
    }

    /// Forgets the last render's references (a new question is rendered)
    pub fn clear(&mut self) {
        self.refs.clear();
    }

    /// What an id of the last render names, as an answer
    pub fn answer(&self, game: &Game, uuid: &str) -> Option<playmat_engine::decide::Answer> {
        if let Some(seat) = self.players.iter().position(|p| p == uuid) {
            return Some(playmat_engine::decide::Answer::Player(seat));
        }
        let r = self.refs.get(uuid)?;
        resolve(game, r)
    }

    /// A reference's id, remembered for the answer
    pub fn of_ref(&mut self, game: &Game, r: &str) -> Option<String> {
        if !is_ref(r) {
            return None;
        }
        if let Some(seat) = r.strip_prefix('P').and_then(|s| s.parse::<usize>().ok()) {
            return self.players.get(seat).cloned();
        }
        let (base, suffix) = split_ref(r);
        let id = if base.starts_with('?') {
            self.uuid(("unplaced", base))
        } else if base == "MD" || base == "MC" {
            self.uuid(("dialog", base))
        } else if let Some(i) = base.strip_prefix('S') {
            // a spell is its card; an ability on the stack is its own object, named by its place from the bottom
            let i: usize = i.parse().ok()?;
            let mut stack: Vec<&playmat_engine::state::StackItem> = game.stack.iter().collect();
            let resolving = game.resolving_item.as_ref();
            if let (Some((at, _)), Some(item)) = (game.resolving, resolving) {
                stack.insert(at.min(stack.len()), item);
            }
            let n = stack.len();
            let item = stack.get(n.checked_sub(1 + i)?)?;
            if item.ability.is_none() && item.delayed.is_none() && suffix.is_empty() {
                self.object(item.card)
            } else {
                self.uuid(("stack", item.card.0, n - 1 - i, suffix))
            }
        } else if base.starts_with('T') {
            self.uuid(("trigger", base))
        } else {
            match resolve(game, base) {
                Some(playmat_engine::decide::Answer::Obj(o)) if suffix.is_empty() => self.object(o),
                Some(playmat_engine::decide::Answer::Obj(o)) => self.uuid(("part", o.0, suffix)),
                _ => self.uuid(("unresolved", r)),
            }
        };
        self.refs.insert(id.clone(), r.to_string());
        Some(id)
    }

    /// A rendered view or prompt with ids for references: values, keys, and `object_id='…'` and `[…]` in its texts
    pub fn map(&mut self, game: &Game, v: Value) -> Value {
        self.map_in(game, v, "")
    }

    fn map_in(&mut self, game: &Game, v: Value, key: &str) -> Value {
        match v {
            Value::String(s) => {
                if NOT_IDS.contains(&key) && !s.contains("object_id=") {
                    return Value::String(s);
                }
                if let Some(id) = self.of_ref(game, &s) {
                    return Value::String(id);
                }
                Value::String(self.text(game, &s))
            }
            Value::Array(a) => Value::Array(a.into_iter().map(|x| self.map_in(game, x, key)).collect()),
            Value::Object(m) => {
                let mut out = Map::new();
                for (k, x) in m {
                    let x = self.map_in(game, x, &k);
                    let k = self.of_ref(game, &k).unwrap_or(k);
                    out.insert(k, x);
                }
                Value::Object(out)
            }
            v => v,
        }
    }

    /// `object_id='B5'` and `[B5]` in a text: the id, and its first three digits as Java's short id
    pub fn text(&mut self, game: &Game, s: &str) -> String {
        if !s.contains("object_id='") && !s.contains('[') {
            return s.to_string();
        }
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(at) = rest.find("object_id='") {
            out.push_str(&rest[..at + 11]);
            rest = &rest[at + 11..];
            let end = rest.find('\'').unwrap_or(rest.len());
            let r = &rest[..end];
            match self.of_ref(game, r) {
                Some(id) => out.push_str(&id),
                None => out.push_str(r),
            }
            rest = &rest[end..];
        }
        out.push_str(rest);
        // short ids
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(at) = rest.find('[') {
            result.push_str(&rest[..at + 1]);
            rest = &rest[at + 1..];
            let Some(end) = rest.find(']') else { break };
            let r = &rest[..end];
            match self.of_ref(game, r) {
                Some(id) => result.push_str(&id[..3]),
                None => result.push_str(r),
            }
            rest = &rest[end..];
        }
        result.push_str(rest);
        result
    }
}

/// Whether a text is a decision-log reference: `P0`, `H0.3`, `L1.20`, `G0.2`, `B12`, `S0`, `E3`, `T0.1`, `?17`,
/// `MD`, `MC`, then any `a2`, `f1`, `h0`, `m1`, `sm0` parts
pub fn is_ref(s: &str) -> bool {
    if s == "MD" || s == "MC" {
        return true;
    }
    let b = s.as_bytes();
    if b.len() < 2 {
        return false;
    }
    let digits = |from: usize| {
        let n = b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
        (n > 0).then_some(from + n)
    };
    let end = match b[0] {
        b'P' | b'B' | b'S' | b'E' | b'?' => digits(1),
        b'H' | b'L' | b'G' | b'T' => digits(1).filter(|&i| b.get(i) == Some(&b'.')).and_then(|i| digits(i + 1)),
        _ => None,
    };
    let Some(mut i) = end else { return false };
    if b[0] == b'P' && i == b.len() {
        return true;
    }
    while i < b.len() {
        let start = if b[i..].starts_with(b"sm") { i + 2 } else { i + 1 };
        if !matches!(b[i], b'a' | b'f' | b'h' | b'm' | b's') {
            return false;
        }
        match digits(start) {
            Some(j) => i = j,
            None => return false,
        }
    }
    true
}

/// A reference's object and the rest (`B4a1` → `B4`, `a1`)
fn split_ref(r: &str) -> (&str, &str) {
    let b = r.as_bytes();
    let mut i = 1;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
        i += 1;
    }
    r.split_at(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references() {
        for r in ["P0", "H0.3", "L1.20", "B12", "S0", "B4a1", "H0.3f1", "S0sm2", "B3a0m1", "T0.1", "?17", "MD"] {
            assert!(is_ref(r), "{r}");
        }
        for r in ["E", "B", "Bears", "P0x", "H0", "M21", "{G}", "B4a", "2"] {
            assert!(!is_ref(r), "{r}");
        }
        assert_eq!(split_ref("B4a1"), ("B4", "a1"));
        assert_eq!(split_ref("H0.3f1"), ("H0.3", "f1"));
    }

    #[test]
    fn uuids_parse() {
        let ids = Ids::new("g", vec![]);
        let u = ids.object(ObjId(7));
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        assert_eq!(u, ids.object(ObjId(7)));
        assert_ne!(u, ids.object(ObjId(8)));
    }
}
