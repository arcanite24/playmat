//! Scorch Spitter: "Whenever Scorch Spitter attacks, it deals 1 damage to the player or planeswalker it's
//! attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ScorchSpitter.ScorchSpitterTriggeredAbility",
        "new AttacksTriggeredAbility(new ScorchSpitterDamageEffect(), false, null, SetTargetPointer.PLAYER)",
        always,
    ),
    Handler::effect("ScorchSpitter.ScorchSpitterDamageEffect", apply),
];

/// `ATTACKER_DECLARED` by this: the base trigger's event is the whole check
fn always(_: &Game, _: &StackItem) -> bool {
    true
}

/// `DamageTargetEffect(1)` pointed at `game.getCombat().getDefenderId(source)`: the planeswalker it attacks, else
/// the defending player (the pointer, kept should it leave combat)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = game.obj(cx.source_id());
    let to = match (o.attacking_pw, cx.player(0)) {
        (Some((pw, zcc)), _) if game.obj(pw).zone == Zone::Battlefield && game.obj(pw).zcc == zcc => Target::Obj(pw),
        (Some(_), _) => return Some(()),
        (None, Some(p)) => Target::Player(p),
        (None, None) => return Some(()),
    };
    deal_damage(game, cx.source_id(), to, 1, false);
    Some(())
}
