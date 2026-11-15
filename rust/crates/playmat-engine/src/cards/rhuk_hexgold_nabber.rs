//! Rhuk, Hexgold Nabber: "Trample, haste. Whenever an equipped creature you control other than Rhuk attacks or
//! dies, you may attach all Equipment attached to that creature to Rhuk."
//!
//! Java's `checkTrigger` falls through from `ATTACKER_DECLARED` to `default: return false`, so only the "dies" half
//! triggers there; this follows it.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "RhukHexgoldNabber.RhukHexgoldNabberTriggeredAbility",
        "new DiesCreatureTriggeredAbility(new RhukHexgoldNabberEffect(), true, StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL)",
        was_equipped,
    ),
    Handler::effect("RhukHexgoldNabber.RhukHexgoldNabberEffect", apply),
];

/// the Equipment attached to the creature that died, as it last was on the battlefield
fn equipment_of(game: &Game, died: ObjId) -> Vec<ObjId> {
    game.obj(died)
        .lki_attachments
        .iter()
        .map(|(a, _)| *a)
        .filter(|a| game.obj(*a).zone == Zone::Battlefield && game.has_subtype(*a, "Equipment"))
        .collect()
}

/// `EquippedPredicate` on the last known information: an Equipment was attached to it
fn was_equipped(game: &Game, item: &StackItem) -> bool {
    item.died.is_some_and(|(o, _)| !equipment_of(game, o).is_empty())
}

/// With Rhuk still on the battlefield, each Equipment that was attached to the creature, still a permanent, is
/// attached to Rhuk (`addAttachment`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let rhuk = cx.source_id();
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let zcc = game.obj(rhuk).zcc;
    for e in equipment_of(game, died) {
        game.attach_to(e, rhuk, zcc);
    }
    Some(())
}
