//! Tormentor's Helm: "Equipped creature gets +1/+1. Whenever equipped creature becomes blocked, it deals 1 damage
//! to defending player. Equip {1}"

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TormentorsHelm.TormentorsHelmTriggeredAbility",
        "new BecomesBlockedAllTriggeredAbility(new TormentorsHelmEffect(null), false, StaticFilters.FILTER_PERMANENT_CREATURE, true)",
        equipped,
    ),
    Handler::effect("TormentorsHelm.TormentorsHelmEffect", apply).args(&["text"]),
];

/// `CREATURE_BLOCKED` of the creature this is attached to (the pointer)
fn equipped(game: &Game, t: &StackItem) -> bool {
    matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Obj(a)) if game.attached(t.card) == Some(*a))
}

/// 1 damage from that creature to the player it attacks (`getDefendingPlayerId`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(a)) = cx.item.targets.first().and_then(|g| g.first()).copied() else { return Some(()) };
    if let Some(p) = game.obj(a).attacking {
        deal_damage(game, a, Target::Player(p), 1, false);
    }
    Some(())
}
