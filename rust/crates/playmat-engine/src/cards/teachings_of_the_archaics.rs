//! Teachings of the Archaics: "If an opponent has more cards in hand than you, draw two cards. Draw three cards
//! instead if an opponent has at least four more cards in hand than you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TeachingsOfTheArchaics.TeachingsOfTheArchaicsEffect", apply)];

/// The most cards in an opponent's hand less yours: above 0, two cards; 4 or more, three
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let most = (0..game.seats())
        .filter(|p| *p != you && !game.player(*p).lost)
        .map(|p| game.player(p).hand.len() as i32)
        .max()
        .unwrap_or(0);
    let diff = most - game.player(you).hand.len() as i32;
    if diff > 0 {
        draw(game, you, if diff >= 4 { 3 } else { 2 });
    }
    Some(())
}
