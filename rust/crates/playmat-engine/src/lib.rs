//! Playmat's Rust rules engine (docs/rust/Engine.md): two-player duels, cards as data, a game state that's cheap to
//! copy for the AI's search. Java stays the oracle: shadow games (`replay`) check this engine against it.

pub mod card;
pub mod decide;
pub mod fingerprint;
pub mod log;
pub mod prompt;
pub mod replay;
pub mod rng;
pub mod rules;
pub mod state;
pub mod view;

/// The cards `rust/tools/transpile.py` wrote, built in.
pub const CARDS_JSON: &str = include_str!("../cards/cards.json");

/// The dungeons players venture into (`card::dungeons`), written by the transpiler beside the cards.
pub const DUNGEONS_JSON: &str = include_str!("../cards/dungeons.json");

/// The cards checked against Java (`cards/verified.txt`, docs/rust/Engine.md "Pool, gate and factory"), one name
/// per line: a game goes to Rust only when every card of both decks is on it.
pub const VERIFIED_TXT: &str = include_str!("../cards/verified.txt");

/// Every card the engine has; cards whose JSON this build doesn't understand are left out. Being in the engine
/// isn't being trusted: see [`is_verified`].
pub fn cards() -> card::CardDb {
    static DB: std::sync::OnceLock<card::CardDb> = std::sync::OnceLock::new();
    DB.get_or_init(|| card::CardDb::load_json(CARDS_JSON).map(|(db, _)| db).unwrap_or_default()).clone()
}

/// The verified cards' names
pub fn verified() -> &'static std::collections::HashSet<&'static str> {
    static SET: std::sync::OnceLock<std::collections::HashSet<&'static str>> = std::sync::OnceLock::new();
    SET.get_or_init(|| VERIFIED_TXT.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect())
}

/// Whether a game with this card may go to Rust: it's on the verified list (and so in the engine)
pub fn is_verified(name: &str) -> bool {
    verified().contains(name)
}

#[cfg(test)]
mod tests {
    /// A verified card must load; a card that's only in the pool may not (yet), and is left out of the engine.
    #[test]
    fn every_verified_card_loads() {
        let (db, errors) = crate::card::CardDb::load_json(crate::CARDS_JSON).unwrap();
        let broken: Vec<&String> =
            errors.iter().filter(|e| crate::is_verified(e.split(": ").next().unwrap_or(""))).collect();
        assert!(broken.is_empty(), "verified cards the engine can't load: {broken:?}");
        if !errors.is_empty() {
            eprintln!(
                "{} cards of the pool the engine can't load yet (left out):\n{}",
                errors.len(),
                errors.join("\n")
            );
        }
        let missing: Vec<&&str> = crate::verified().iter().filter(|n| db.get(n).is_none()).collect();
        assert!(missing.is_empty(), "verified cards not in cards.json: {missing:?}");
    }
}
