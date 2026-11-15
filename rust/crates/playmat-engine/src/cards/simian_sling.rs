//! Simian Sling: "Equipped creature gets +1/+1. Whenever Simian Sling or equipped creature becomes blocked, it deals
//! 1 damage to defending player. Reconfigure {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "SimianSling.SimianSlingTriggeredAbility",
        "new BecomesBlockedAllTriggeredAbility(new SimianSlingEffect(), false, StaticFilters.FILTER_PERMANENT_CREATURE, true)",
        sling_or_equipped,
    ),
    Handler::effect("SimianSling.SimianSlingEffect", apply),
];

/// `CREATURE_BLOCKED` of the Sling itself or of the creature it's attached to (as it is, or as it last was)
fn sling_or_equipped(game: &Game, item: &StackItem) -> bool {
    let Some(Target::Obj(attacker)) = item.targets.first().and_then(|t| t.first()).copied() else {
        return false;
    };
    if attacker == item.card {
        return true;
    }
    let sling = game.obj(item.card);
    let attached = if sling.zone == Zone::Battlefield { sling.attached_to } else { sling.lki_attached_to };
    attached.is_some_and(|(a, _)| a == attacker)
}

/// The blocked creature deals 1 damage to the player it's attacking
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(attacker)) = cx.item.targets.first().and_then(|t| t.first()).copied() else {
        return None;
    };
    let defender = game.opponent(game.obj(attacker).controller);
    deal_damage(game, attacker, Target::Player(defender), 1, false);
    Some(())
}
