//! Angrath, Minotaur Pirate: "+2: Angrath deals 1 damage to target opponent or planeswalker and each creature that
//! player or that planeswalker's controller controls. −3: Return target Pirate card from your graveyard to the
//! battlefield. −11: Destroy all creatures target opponent controls. Angrath deals damage to that player equal to
//! their total power."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AngrathMinotaurPirate.AngrathMinotaurPirateThirdAbilityEffect", apply)];

/// As Java does it: `getAllActivePermanents(filter, source.getSourceId(), game)` asks for the creatures *Angrath*
/// controls (a permanent's id as the controller), so nothing is destroyed and the opponent is dealt 0 damage
fn apply(_: &mut Game, _: &mut dyn Decider, _: &EffectCx) -> Option<()> {
    Some(())
}
