//! Molten Primordial: "Haste. When Molten Primordial enters, for each opponent, gain control of up to one target
//! creature that player controls until end of turn. Untap those creatures. They gain haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MoltenPrimordial.MoltenPrimordialEffect", apply).effects(&[
    "new GainControlTargetEffect(Duration.EndOfTurn)",
    "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.EndOfTurn)",
    "new UntapTargetEffect()",
])];

/// Each target (one per opponent, `ForEachPlayerTargetsAdjuster`), in order: control until end of turn, haste until
/// end of turn, then untapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for i in 0..cx.legal.len() {
        let Some(o) = cx.obj(i) else { continue };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
        cx.run_on(game, d, &cx.effects[1..2], vec![Target::Obj(o)]);
        cx.run_on(game, d, &cx.effects[2..3], vec![Target::Obj(o)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
