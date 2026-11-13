//! Gruul Ragebeast: "Whenever Gruul Ragebeast or another creature you control enters, that creature fights target
//! creature an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "GruulRagebeast.GruulRagebeastTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility(new GruulRagebeastEffect(), StaticFilters.FILTER_CONTROLLED_CREATURE)",
        check,
    ),
    Handler::effect("GruulRagebeast.GruulRagebeastEffect", apply),
];

/// `checkTrigger` beyond the event: the source is still on the battlefield as the creature enters
fn check(_game: &Game, t: &StackItem) -> bool {
    t.entering.is_some()
}

/// The creature that entered (the effect's fixed target) fights the ability's target, both still creatures on the
/// battlefield (`Permanent.fight`, as `FightTargetsEffect` does it)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(a), Some(b)) = (cx.item.entering, cx.obj(0)) else { return Some(()) };
    let creature = |g: &Game, o: ObjId| g.obj(o).zone == Zone::Battlefield && g.has_type(o, CardType::Creature);
    if creature(game, a) && creature(game, b) {
        let pb = game.power_toughness(b).0;
        let pa = game.power_toughness(a).0;
        deal_damage(game, b, Target::Obj(a), pb, false);
        let pa = pa.min(fight_lethal(game, b, a));
        deal_damage(game, a, Target::Obj(b), pa, false);
    }
    Some(())
}
