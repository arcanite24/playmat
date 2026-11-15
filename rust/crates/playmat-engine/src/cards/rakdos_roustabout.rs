//! Rakdos Roustabout: "Whenever Rakdos Roustabout becomes blocked, it deals 1 damage to the player or planeswalker
//! it's attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "RakdosRoustabout.RakdosRoustaboutAbility",
        "new BecomesBlockedSourceTriggeredAbility(new RakdosRoustaboutDefenderDamageEffect(), false)",
        becomes_blocked,
    ),
    Handler::effect("RakdosRoustabout.RakdosRoustaboutDefenderDamageEffect", apply),
];

/// `CREATURE_BLOCKED` for this creature: the base trigger is all of it
fn becomes_blocked(_: &Game, _: &StackItem) -> bool {
    true
}

/// `DamageTargetEffect(1)` pointed at `getCombat().getDefenderId(this)`: the planeswalker it attacks while it's
/// still that object, else the player
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = game.obj(cx.source_id());
    let to = match (src.attacking_pw, src.attacking) {
        (Some((pw, zcc)), _) => {
            if game.obj(pw).zone != Zone::Battlefield || game.obj(pw).zcc != zcc {
                return Some(());
            }
            Target::Obj(pw)
        }
        (None, Some(p)) => Target::Player(p),
        _ => return Some(()),
    };
    deal_damage(game, cx.source_id(), to, 1, false);
    Some(())
}
