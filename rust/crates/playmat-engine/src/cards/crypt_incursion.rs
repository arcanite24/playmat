//! Crypt Incursion: "Exile all creature cards from target player's graveyard. You gain 3 life for each card exiled
//! this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CryptIncursion.CryptIncursionEffect", apply)];

/// The creature cards there, exiled together; 3 life for each one now in exile (the gain is made even for none)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let cards: Vec<ObjId> =
        game.player(p).graveyard.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let n = cards.iter().filter(|c| game.obj(**c).zone == Zone::Exile).count() as i32;
    gain_life(game, cx.controller(), 3 * n);
    Some(())
}
