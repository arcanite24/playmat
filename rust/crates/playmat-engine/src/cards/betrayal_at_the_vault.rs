//! Betrayal at the Vault: "Target creature you control deals damage equal to its power to each of two other target
//! creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BetrayalAtTheVault.BetrayalAtTheVaultEffect", apply)];

/// the first target's power, read again before each one (`creature.getPower().getValue()`), to each of the second
/// target's creatures still there, in the order chosen
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    for p in cx.objs(1) {
        if game.obj(p).zone != Zone::Battlefield {
            continue;
        }
        let power = game.power_toughness(creature).0;
        deal_damage(game, creature, Target::Obj(p), power, false);
    }
    Some(())
}
