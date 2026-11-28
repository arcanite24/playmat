//! Beluna Grandsquall // Seek Thrills: "Mill seven cards. Then put all cards that have an Adventure from among the
//! milled cards into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BelunaGrandsquall.SeekThrillsEffect", apply)];

/// `millCards(7)`, then the milled cards still in the graveyard that have an Adventure (`AdventurePredicate`) to the
/// hand, in the order milled
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    for c in top {
        let o = game.obj(c);
        if o.zone == Zone::Graveyard && (o.def.adventure.is_some() || o.adventurer.is_some()) {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}
