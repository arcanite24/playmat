//! Magnetic Snuffler: "When Magnetic Snuffler enters, return target Equipment card from your graveyard to the
//! battlefield attached to Magnetic Snuffler. Whenever you sacrifice an artifact, put a +1/+1 counter on Magnetic
//! Snuffler."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MagneticSnuffler.MagneticSnufflerEffect", apply)
    .effects(&["new ReturnFromGraveyardToBattlefieldTargetEffect()"])];

/// `super.apply` (onto the battlefield), then `creature.addAttachment(equipment)` when both are permanents
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let equipment = cx.obj(0);
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let equipment = equipment?;
    let creature = cx.source_id();
    if game.obj(equipment).zone != Zone::Battlefield || game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(creature).zcc;
    game.attach_to(equipment, creature, zcc);
    Some(())
}
