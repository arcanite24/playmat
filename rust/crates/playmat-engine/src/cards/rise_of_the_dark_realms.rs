//! Rise of the Dark Realms: "Put all creature cards from all graveyards onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiseOfTheDarkRealms.RiseOfTheDarkRealmsEffect", apply)];

/// The creature cards of each graveyard, players in turn order from you, each graveyard in its order; then
/// `moveCards` puts them onto the battlefield in that order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cards.extend(game.player(p).graveyard.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)));
    }
    for c in cards {
        if game.obj(c).zone == Zone::Graveyard {
            enter_battlefield(game, d, c, you, false);
        }
    }
    Some(())
}
