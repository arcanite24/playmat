//! Views (docs/rust/Views.md): what the web client sees, as Java's `JsonCodec` writes a `GameView`, with ids as
//! decision-log references, and the field-by-field comparison with Java's.

use crate::state::{Game, Seat, Step};
use serde_json::{json, Value};

/// The view `seat`'s client gets of the game, as Java's `GameSessionPlayer.prepareGameView` builds it and `JsonCodec`
/// writes it, with ids as decision-log references. `names` are the players' names, by seat.
pub fn game_view(game: &Game, seat: Seat, names: &[&str]) -> Value {
    let name = |s: Option<Seat>| s.and_then(|s| names.get(s)).map(|n| n.to_string()).unwrap_or_default();
    let players: Vec<Value> = (0..game.seats()).map(|p| player_view(game, p, seat, names)).collect();
    json!({
        "players": players,
        "myPlayerId": format!("P{}", seat),
        "phase": game.step.map(phase_of).unwrap_or(""),
        "step": game.step.map(|s| s.name()).unwrap_or(""),
        "turn": game.turn,
        "activePlayerId": game.active.map(|a| format!("P{}", a)),
        "activePlayerName": name(game.active),
        "priorityPlayerName": name(game.priority),
        "special": false,
        "rollbackTurnsAllowed": false,
        "attackOption": "LEFT",
        "rangeOfInfluence": "ALL",
        "myHand": {},
        "myHelperEmblems": {},
        "opponentHands": {},
        "watchedHands": {},
        "stack": {},
        "combat": [],
        "revealed": [],
        "lookedAt": [],
        "companion": [],
    })
}

/// `TurnPhase` of a step
fn phase_of(step: Step) -> &'static str {
    match step {
        Step::Untap | Step::Upkeep | Step::Draw => "BEGINNING",
        Step::PrecombatMain => "PRECOMBAT_MAIN",
        Step::BeginCombat
        | Step::DeclareAttackers
        | Step::DeclareBlockers
        | Step::FirstCombatDamage
        | Step::CombatDamage
        | Step::EndCombat => "COMBAT",
        Step::PostcombatMain => "POSTCOMBAT_MAIN",
        Step::EndTurn | Step::Cleanup => "END",
    }
}

/// `PlayerView`: seat `p` as `viewer` sees it
fn player_view(game: &Game, p: Seat, viewer: Seat, names: &[&str]) -> Value {
    let pl = game.player(p);
    let mut pool = pl.mana_pool;
    for (m, _, _) in &pl.conditional_pool {
        for k in 0..5 {
            pool.colored[k] += m.colored[k];
        }
        pool.colorless += m.colorless;
    }
    json!({
        "playerId": format!("P{}", p),
        "name": names.get(p).copied().unwrap_or(""),
        "controlled": p == viewer,
        "isHuman": true,
        "life": pl.life,
        "counters": pl.counters.iter().map(|(n, c)| json!({"name": n, "count": c})).collect::<Vec<_>>(),
        "wins": 0,
        "winsNeeded": 0,
        "libraryCount": pl.library.len(),
        "handCount": pl.hand.len(),
        "isActive": game.active == Some(p),
        "hasPriority": game.priority == Some(p),
        "hasLeft": false,
        "manaPool": {
            "white": pool.colored[0], "blue": pool.colored[1], "black": pool.colored[2],
            "red": pool.colored[3], "green": pool.colored[4], "colorless": pool.colorless,
        },
        "attachments": [],
        "commandList": [],
        "designationNames": [],
        "helperCards": {},
        "sideboard": {},
        "passedTurn": false,
        "passedUntilEndOfTurn": false,
        "passedUntilNextMain": false,
        "passedUntilStackResolved": false,
        "passedAllTurns": false,
        "passedUntilEndStepBeforeMyTurn": false,
        "monarch": false,
        "initiative": false,
    })
}

/// Fields left out of the comparison, by name anywhere in the view: they can't match by design.
pub const IGNORED: &[(&str, &str)] = &[
    ("priorityTime", "the priority timer"),
    ("bufferTime", "the priority timer"),
    ("priorityTimeLeftSecs", "the priority timer"),
    ("priorityTimeSavedTimeMs", "the priority timer"),
    ("bufferTimeLeft", "the priority timer"),
    ("timerActive", "the priority timer"),
    ("totalErrorsCount", "Java's own error counters"),
    ("totalEffectsCount", "Java's own effect counters"),
    ("gameCycle", "Java's own state counter"),
    ("userData", "the player's settings, which the host passes through"),
    ("statesSavedSize", "Java's rollback states"),
];

/// One field that differs: its path (`players[0].battlefield{B3}.tapped`), Java's value and the engine's.
#[derive(Clone, Debug, PartialEq)]
pub struct Difference {
    pub path: String,
    pub java: String,
    pub rust: String,
}

/// The path with list indexes and map keys left out (`players[].battlefield{}.tapped`), to count differences by field.
pub fn field_of(path: &str) -> String {
    let mut out = String::new();
    let mut skip: Option<char> = None;
    for c in path.chars() {
        match (skip, c) {
            (None, '[') => {
                skip = Some(']');
                out.push('[');
            }
            (None, '{') => {
                skip = Some('}');
                out.push('{');
            }
            (Some(end), c) if c == end => {
                skip = None;
                out.push(c);
            }
            (Some(_), _) => {}
            (None, c) => out.push(c),
        }
    }
    out
}

/// A map key that is an object's reference (`B3`, `H0.2`, `?`), not a field name.
fn is_ref(key: &str) -> bool {
    let mut c = key.chars();
    key == "?" || (c.next().is_some_and(|c| c.is_ascii_uppercase()) && c.next().is_some_and(|c| c.is_ascii_digit()))
}

fn ignored(key: &str) -> bool {
    IGNORED.iter().any(|(k, _)| *k == key)
}

fn short(v: Option<&Value>) -> String {
    match v {
        None => "(missing)".into(),
        Some(v) => {
            let s = v.to_string();
            if s.len() > 200 {
                format!("{}…", &s[..s.char_indices().nth(200).map(|(i, _)| i).unwrap_or(s.len())])
            } else {
                s
            }
        }
    }
}

/// An object's key in a list: its `id` when it has one.
fn list_key(v: &Value) -> Option<String> {
    v.get("id").and_then(|i| i.as_str()).map(|s| s.to_string())
}

/// Every field where the two views differ. Maps match by key; lists of objects with ids by id, other lists by place.
pub fn compare(java: &Value, rust: &Value) -> Vec<Difference> {
    let mut out = Vec::new();
    diff(&mut out, String::new(), Some(java), Some(rust));
    out
}

fn diff(out: &mut Vec<Difference>, path: String, java: Option<&Value>, rust: Option<&Value>) {
    // Gson leaves out nulls: a null is the same as a missing field
    let java = java.filter(|v| !v.is_null());
    let rust = rust.filter(|v| !v.is_null());
    match (java, rust) {
        (None, None) => {}
        (Some(Value::Object(a)), Some(Value::Object(b))) => {
            let mut keys: Vec<&String> = a.keys().collect();
            for k in b.keys() {
                if !a.contains_key(k) {
                    keys.push(k);
                }
            }
            for k in keys {
                if ignored(k) {
                    continue;
                }
                let p = if is_ref(k) {
                    format!("{}{{{}}}", path, k)
                } else if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                diff(out, p, a.get(k), b.get(k));
            }
        }
        (Some(Value::Array(a)), Some(Value::Array(b))) => {
            let keyed = !a.is_empty() && a.iter().chain(b.iter()).all(|v| list_key(v).is_some());
            if keyed {
                let mut keys: Vec<String> = a.iter().filter_map(list_key).collect();
                for k in b.iter().filter_map(list_key) {
                    if !keys.contains(&k) {
                        keys.push(k);
                    }
                }
                for k in keys {
                    let x = a.iter().find(|v| list_key(v).as_deref() == Some(&k));
                    let y = b.iter().find(|v| list_key(v).as_deref() == Some(&k));
                    diff(out, format!("{}[{}]", path, k), x, y);
                }
            } else if a.len() != b.len() || a.iter().any(|v| v.is_object() || v.is_array()) {
                for i in 0..a.len().max(b.len()) {
                    diff(out, format!("{}[{}]", path, i), a.get(i), b.get(i));
                }
            } else if a != b {
                out.push(Difference { path, java: short(java), rust: short(rust) });
            }
        }
        (a, b) if a == b => {}
        (a, b) => out.push(Difference { path, java: short(a), rust: short(b) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_by_field() {
        let java = json!({"turn": 3, "players": [{"life": 20, "battlefield": {"B0": {"tapped": true}}}],
                          "priorityTime": 5, "stack": {}, "gone": null});
        let rust = json!({"turn": 3, "players": [{"life": 18, "battlefield": {"B0": {"tapped": false}}}],
                          "priorityTime": 9, "stack": {}});
        let d = compare(&java, &rust);
        let paths: Vec<&str> = d.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths, vec!["players[0].life", "players[0].battlefield{B0}.tapped"]);
        assert_eq!(field_of("players[0].hand{H0.1}.rules[2]"), "players[].hand{}.rules[]");
    }

    #[test]
    fn lists_with_ids_match_by_id() {
        let java = json!({"combat": [{"id": "B1", "x": 1}, {"id": "B2", "x": 2}]});
        let rust = json!({"combat": [{"id": "B2", "x": 2}, {"id": "B1", "x": 1}]});
        assert!(compare(&java, &rust).is_empty());
    }
}
