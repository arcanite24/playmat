//! Kardur, Doomscourge: "When Kardur enters, until your next turn, creatures your opponents control attack each
//! combat if able and attack a player other than you if able."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KardurDoomscourge.KardurDoomscourgeEffect", attack_another_player)];

/// `KardurDoomscourgeEffect.canAttack`: with only two players left any defender is fine, so in a duel it changes
/// nothing
fn attack_another_player(_game: &mut Game, _d: &mut dyn Decider, _cx: &EffectCx) -> Option<()> {
    Some(())
}
