//! Inner Struggle: "Target creature deals damage to itself equal to its power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InnerStruggle.InnerStruggleEffect", apply)];

/// `targetCreature.damage(power, targetCreature.getId(), …)`: the creature is the damage's source
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(o).0;
    if power > 0 {
        deal_damage(game, o, Target::Obj(o), power, false);
    }
    Some(())
}
