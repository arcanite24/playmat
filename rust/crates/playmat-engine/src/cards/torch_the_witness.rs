//! Torch the Witness: "Torch the Witness deals twice X damage to target creature. If excess damage was dealt to that
//! creature this way, investigate."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TorchTheWitness.TorchTheWitnessEffect", apply).effects(&["new InvestigateEffect()"])];

/// `damageWithExcess(2 * X)`; any excess, investigate
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let excess = super::damage_with_excess_effect::damage_with_excess(game, cx.source_id(), o, 2 * cx.item.x);
    if excess > 0 {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}
