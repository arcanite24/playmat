//! `IncrementAbility`: "Whenever you cast a spell, if the amount of mana you spent is greater than this creature's
//! power or toughness, put a +1/+1 counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "IncrementAbility",
    "new SpellCastControllerTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_SPELL, false)",
    check,
)];

/// `checkInterveningIfClause`, as it triggers and as it resolves: the mana spent on the spell is more than the lower
/// of this permanent's power and toughness (no permanent: false)
fn check(game: &Game, t: &StackItem) -> bool {
    let o = game.obj(t.card);
    if o.zone != Zone::Battlefield || t.ability.is_some_and(|(_, zcc)| zcc != o.zcc) {
        return false;
    }
    let Some(spell) = t.cast_spell else { return false };
    let (p, tough) = game.power_toughness(t.card);
    game.obj(spell).mana_spent as i32 > p.min(tough)
}
