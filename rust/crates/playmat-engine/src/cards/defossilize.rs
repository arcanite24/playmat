//! Defossilize: "Return target creature card from your graveyard to the battlefield. That creature explores, then it
//! explores again."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Defossilize.DefossilizeEffect", apply)
    .effects(&["new ReturnFromGraveyardToBattlefieldTargetEffect()"])];

/// onto the battlefield, then `explorePermanent(…, 2)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if game.obj(c).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    for _ in 0..2 {
        if !explore(game, d, you, c) {
            return None;
        }
    }
    Some(())
}
