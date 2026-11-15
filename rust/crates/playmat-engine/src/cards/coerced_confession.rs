//! Coerced Confession: "Target player mills four cards. You draw a card for each creature card put into their
//! graveyard this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CoercedConfession.CoercedConfessionMillEffect", apply)];

/// the milled cards that are creature cards in a graveyard afterwards
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let top: Vec<ObjId> = game.player(p).library.iter().take(4).copied().collect();
    cx.run(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(4) }]);
    if d.stopped() {
        return None;
    }
    let n = top
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Graveyard && game.obj(**c).is(CardType::Creature))
        .count();
    if n > 0 {
        draw(game, cx.controller(), n);
    }
    Some(())
}
