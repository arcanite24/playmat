//! Archon of Redemption: "Whenever Archon of Redemption or another creature with flying you control enters, you may
//! gain life equal to that creature's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArchonOfRedemption.ArchonOfRedemptionEffect", apply)];

/// The creature that entered (the pointer, `getFirstTargetPermanentOrLKI`): life equal to its power
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(o)) = cx.item.targets.first().and_then(|t| t.first()).copied() else { return Some(()) };
    let power = game.power_toughness(o).0;
    if power > 0 {
        gain_life(game, cx.controller(), power);
    }
    Some(())
}
