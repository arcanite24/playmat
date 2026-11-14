//! Malcolm, Alluring Scoundrel: "Whenever Malcolm deals combat damage to a player, put a chorus counter on it. Draw
//! a card, then discard a card. If there are four or more chorus counters on Malcolm, you may cast the discarded card
//! without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MalcolmAlluringScoundrel.MalcolmAlluringScoundrelEffect", apply)];

/// Draw, discard (`discard(1, false, false)`), then with four chorus counters on Malcolm (still that permanent)
/// `castSpellWithAttributesForFree` of the discarded card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let (src, zcc) = cx.source;
    let chorus = if game.obj(src).zcc == zcc && game.obj(src).zone == Zone::Battlefield {
        game.obj(src).counter("chorus")
    } else {
        0
    };
    if let (Some(&card), true) = (discarded.first(), chorus >= 4) {
        super::breaching_dragonstorm::cast_for_free(game, d, you, card, |_, _| true)?;
    }
    Some(())
}
