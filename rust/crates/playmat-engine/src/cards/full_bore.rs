//! Full Bore: "Target creature you control gets +3/+2 until end of turn. If that creature was cast for its warp cost,
//! it also gains trample and haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FullBore.FullBoreEffect", apply).effects(&[
    "new GainAbilityTargetEffect(TrampleAbility.getInstance(), Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
])];

/// `WarpAbility.checkIfPermanentWarped`: the permanent was cast for its warp cost
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield || !game.obj(o).warped {
        return Some(());
    }
    cx.run_on(game, d, cx.effects, vec![Target::Obj(o)]);
    (!d.stopped()).then_some(())
}
