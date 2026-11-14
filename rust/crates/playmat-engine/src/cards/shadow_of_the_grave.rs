//! Shadow of the Grave: "Return to your hand all cards in your graveyard that you cycled or discarded this turn."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShadowOfTheGrave.ShadowOfTheGraveEffect", apply)];

/// `CardsCycledOrDiscardedThisTurnWatcher` (a cycled card was discarded too): each card you discarded this turn
/// that's still in your graveyard as that card, in the order discarded, to your hand one at a time
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game
        .discarded_this_turn
        .iter()
        .filter(|(c, zcc)| {
            let o = game.obj(*c);
            o.zone == Zone::Graveyard && o.zcc == *zcc && o.owner == you
        })
        .map(|(c, _)| *c)
        .collect();
    for c in cards {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
