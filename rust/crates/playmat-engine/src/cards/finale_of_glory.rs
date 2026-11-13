//! Finale of Glory: "Create X 2/2 white Soldier creature tokens with vigilance. If X is 10 or more, also create X
//! 4/4 white Angel creature tokens with flying and vigilance."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FinaleOfGlory.FinaleOfGloryEffect", apply).effects(&[
    "new CreateTokenEffect(new SoldierVigilanceToken(), GetXValue.instance)",
    "new CreateTokenEffect(new AngelVigilanceToken(), GetXValue.instance)",
])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x <= 0 {
        return Some(());
    }
    cx.run(game, d, &cx.effects[..1]);
    if x >= 10 {
        cx.run(game, d, &cx.effects[1..]);
    }
    Some(())
}
