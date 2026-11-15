//! Sire of Insanity: "At the beginning of each end step, each player discards their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SireOfInsanity.SireOfInsanityEffect", apply)];

/// each player in turn order from the controller discards their whole hand, one batch each
/// (`player.discard(hand, false, source, game)`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let hand = game.player(p).hand.clone();
        for c in hand {
            discard_card(game, p, c);
        }
        game.discarded_cards();
    }
    Some(())
}
