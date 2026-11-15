//! Hedron Blade: "Equipped creature gets +1/+1. Whenever equipped creature becomes blocked by one or more colorless
//! creatures, it gains deathtouch until end of turn. Equip {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "HedronBlade.HedronBladeTriggeredAbility",
    "new BecomesBlockedAllTriggeredAbility($0, false, StaticFilters.FILTER_PERMANENT_CREATURE, true)",
    by_colorless,
)];

/// The attacker that became blocked (the pointer) is the creature this is attached to, and a colorless creature
/// blocks it (`DECLARED_BLOCKERS`, the equipped creature's combat group)
fn by_colorless(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Obj(a)) = t.targets.first().and_then(|g| g.first()).copied() else { return false };
    game.attached(t.card) == Some(a)
        && game.obj(a).blocked_by.iter().any(|b| {
            game.obj(*b).zone == Zone::Battlefield && game.has_type(*b, CardType::Creature) && game.colors(*b).is_empty()
        })
}
