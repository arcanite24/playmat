//! Liliana Vess: "−8: Put all creature cards from all graveyards onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianaVess.LilianaVessEffect", apply)];

/// The creature cards of each graveyard of the players in range, from you in turn order, onto the battlefield
/// together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cards.extend(game.player(p).graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)));
    }
    if !cards.is_empty() && !put_cards(game, d, you, &cards, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
