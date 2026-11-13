//! Akoum Hellkite: "Landfall — Whenever a land you control enters, Akoum Hellkite deals 1 damage to any target. If that
//! land is a Mountain, Akoum Hellkite deals 2 damage instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AkoumHellkite.AkoumHellkiteTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility(new AkoumHellkiteDamageEffect(), StaticFilters.FILTER_CONTROLLED_PERMANENT_LAND)",
        any,
    ),
    Handler::effect("AkoumHellkite.AkoumHellkiteDamageEffect", damage),
];

fn any(_: &Game, _: &StackItem) -> bool {
    true
}

/// the land that entered (its last known subtypes) sets the damage
fn damage(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(land) = cx.item.entering else { return Some(()) };
    let Some(to) = cx.target(0) else { return Some(()) };
    let n = if game.has_subtype(land, "Mountain") { 2 } else { 1 };
    deal_damage(game, cx.source_id(), to, n, false);
    Some(())
}
