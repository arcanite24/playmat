//! Playmat's Rust rules engine (docs/rust/Engine.md): two-player duels, cards as data, a game state that's cheap to
//! copy for the AI's search. Java stays the oracle: shadow games (`replay`) check this engine against it.

pub mod card;
pub mod decide;
pub mod fingerprint;
pub mod replay;
pub mod rng;
pub mod rules;
pub mod state;

/// The cards `rust/tools/transpile.py` wrote, built in.
pub const CARDS_JSON: &str = include_str!("../cards/cards.json");

/// Every card the engine has; cards whose JSON this build doesn't understand are left out.
pub fn cards() -> card::CardDb {
    card::CardDb::load_json(CARDS_JSON).map(|(db, _)| db).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_transpiled_card_loads() {
        let (_, errors) = crate::card::CardDb::load_json(crate::CARDS_JSON).unwrap();
        assert!(errors.is_empty(), "cards.json has cards the engine can't load:\n{}", errors.join("\n"));
    }
}
