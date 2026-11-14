//! Justice Strike: "Target creature deals damage to itself equal to its power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JusticeStrike.JusticeStrikeEffect", apply)];

/// The creature is the damage's source (`permanent.damage(power, permanent.getId(), …)`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(t).0;
    if power > 0 {
        deal_damage(game, t, Target::Obj(t), power, false);
    }
    Some(())
}
