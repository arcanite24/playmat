//! Damping Sphere: "If a land is tapped for two or more mana, it produces {C} instead of any other type and amount.
//! Each spell a player casts costs {1} more to cast for each other spell that player has cast this turn."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "DampingSphere.DampingSphereReplacementEffect",
        &[EventType::TappedForMana],
        land_for_two,
        colorless,
    ),
    Handler::cost("DampingSphere.DampingSphereIncreasementAllEffect", CostKind::Increase, spell, more),
];

/// `TAPPED_FOR_MANA` of a land, for more than one mana
fn land_for_two(game: &Game, _: &StaticCx, ev: &Event) -> bool {
    ev.target.is_some_and(|l| game.has_type(l, CardType::Land)) && ev.amount > 1
}

/// `mana.setToMana(Mana.ColorlessMana(1))`
fn colorless(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.colorless = Some(1);
    false
}

/// every player's spells (`SpellsCostIncreasingAllEffect(1, new FilterCard(), TargetController.ANY)`)
fn spell(_: &Game, _: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
}

/// {1} more for each spell its caster has cast this turn (`CastSpellLastTurnWatcher`)
fn more(game: &Game, _: &StaticCx, of: &CostCx, cost: &mut ManaCost) {
    cost.generic += game.watch.spells.iter().filter(|s| s.caster == of.seat).count() as u32;
}
