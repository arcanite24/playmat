//! Sigarda's Aid: "Whenever an Equipment you control enters, you may attach it to target creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SigardasAid.SigardasAidEffect", apply)];

/// The Equipment that entered (the trigger's target pointer) is attached to the target creature, if both are still
/// on the battlefield (the "you may" is the trigger's)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(equipment), Some(creature)) = (cx.item.entering, cx.obj(0)) else { return Some(()) };
    if game.obj(equipment).zone == Zone::Battlefield
        && game.obj(creature).zone == Zone::Battlefield
        && equipment != creature
        && !game.has_type(equipment, CardType::Creature)
    {
        let z = game.obj(creature).zcc;
        game.attach_to(equipment, creature, z);
    }
    Some(())
}
