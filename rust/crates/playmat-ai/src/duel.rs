//! New two-player games from deck lists, for the bench and AI-against-AI games.

use playmat_engine::card::CardDb;
use playmat_engine::rng::JavaRandom;
use playmat_engine::rules;
use playmat_engine::state::{Game, Step, Zone};
use std::sync::Arc;

/// A deck: a name and its main deck's card names, one per card.
#[derive(Clone, Debug)]
pub struct Deck {
    pub name: String,
    pub cards: Vec<String>,
}

impl Deck {
    /// A Career deck file: `# name: …` and `<count> <card name>` lines; a sideboard (`SB:` lines, or after a
    /// `Sideboard` line) is left out.
    pub fn parse(name: &str, text: &str) -> Deck {
        let mut deck = Deck { name: name.to_string(), cards: Vec::new() };
        for line in text.lines().map(str::trim) {
            if let Some(n) = line.strip_prefix("# name:") {
                deck.name = n.trim().to_string();
                continue;
            }
            if line.eq_ignore_ascii_case("sideboard") || line.starts_with("SB") {
                break;
            }
            let Some((count, card)) = line.split_once(' ') else { continue };
            let Ok(count) = count.trim_end_matches('x').parse::<usize>() else { continue };
            for _ in 0..count {
                deck.cards.push(card.trim().to_string());
            }
        }
        deck
    }

    /// The cards the engine doesn't have
    pub fn missing(&self, db: &CardDb) -> Vec<String> {
        let mut out: Vec<String> = self.cards.iter().filter(|c| db.get(c).is_none()).cloned().collect();
        out.dedup();
        out
    }
}

/// A duel ready for its first priority: libraries shuffled with `seed`, seven cards each, `first` starting.
pub fn new_duel(db: &CardDb, decks: [&Deck; 2], seed: i64, first: usize) -> Result<Game, String> {
    let mut game = Game::new(2, JavaRandom::new(seed).state());
    for (seat, deck) in decks.iter().enumerate() {
        let mut library = Vec::new();
        for name in &deck.cards {
            let def = db.get(name).ok_or_else(|| format!("card not in the engine: {name}"))?;
            library.push(game.create_obj(Arc::clone(&def), seat, Zone::Library));
        }
        game.rng.shuffle(&mut library);
        game.player_mut(seat).library = library;
    }
    for seat in 0..2 {
        rules::draw(&mut game, seat, 7);
    }
    game.turn = 1;
    game.active = Some(first);
    game.priority = Some(first);
    game.step = Some(Step::Upkeep);
    Ok(game)
}
