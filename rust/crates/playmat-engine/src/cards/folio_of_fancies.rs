//! Folio of Fancies: "{2}{U}, {T}: Each opponent mills cards equal to the number of cards in their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FolioOfFancies.FolioOfFanciesEffect", apply)];

/// each opponent, in turn order from you, `millCards(hand size)`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let n = game.player(p).hand.len();
        let top: Vec<ObjId> = game.player(p).library.iter().take(n).copied().collect();
        note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
        for c in top {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}
