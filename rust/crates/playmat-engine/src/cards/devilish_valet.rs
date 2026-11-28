//! Devilish Valet: "Trample, haste. Whenever another creature you control enters, double Devilish Valet's power until
//! end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DevilishValet.DevilishValetEffect", apply)];

/// +X/+0 until end of turn, X its power now, while it's still the permanent the ability came from
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let src = cx.source_id();
    let power = game.power_toughness(src).0;
    boost(game, src, power, 0, Duration::EndOfTurn);
    Some(())
}
