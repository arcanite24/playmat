//! Wildcall: "Manifest the top card of your library, then put X +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Wildcall.WildcallEffect", apply)];

/// the top card, manifested (`ManifestEffect(1)`), then X +1/+1 counters on it as the permanent it became
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    cx.run(game, d, &[EffectDef::ManifestEffect { amount: Amount::Fixed(1), cloak: false }]);
    if d.stopped() {
        return None;
    }
    let x = cx.item.x;
    if x > 0 && game.obj(card).zone == Zone::Battlefield {
        put_counters(game, card, "+1/+1", x, you);
    }
    Some(())
}
