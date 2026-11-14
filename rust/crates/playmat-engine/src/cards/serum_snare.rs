//! Serum Snare: "Return target nonland permanent to its owner's hand. If that permanent had mana value 3 or less,
//! proliferate."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SerumSnare.SerumSnareEffect", apply)
    .effects(&["new ReturnToHandTargetEffect()", "new ProliferateEffect()"])];

/// Its mana value as it was, the bounce, then the proliferate
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let mv = game.obj(o).def.mana_value();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if mv <= 3 {
        cx.run(game, d, &cx.effects[1..]);
    }
    Some(())
}
