//! Yip Yip!: "Target creature you control gets +2/+2 until end of turn. If that creature is an Ally, it also gains
//! flying until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("YipYip.YipYipEffect", apply)
    .effects(&["new GainAbilityTargetEffect(FlyingAbility.getInstance())"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.has_subtype(o, "Ally") {
        cx.run(game, d, cx.effects);
    }
    Some(())
}
