//! Thousand-Year Storm: "Whenever you cast an instant or sorcery spell, copy it for each other instant and sorcery
//! spell you've cast before it this turn. You may choose new targets for the copies."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ThousandYearStorm.ThousandYearStormAbility",
        "new SpellCastControllerTriggeredAbility(new ThousandYearStormEffect(), StaticFilters.FILTER_SPELL_AN_INSTANT_OR_SORCERY, false, SetTargetPointer.SPELL)",
        always,
    ),
    Handler::effect("ThousandYearStorm.ThousandYearStormEffect", apply),
];

fn always(_: &Game, _: &StackItem) -> bool {
    true
}

/// `ThousandYearStormWatcher`: the instant and sorcery spells `you` cast this turn up to `spell` (it counted too),
/// less one
fn storm_count(game: &Game, you: Seat, spell: ObjId) -> i32 {
    let mut n = 0;
    for s in game.watch.spells.iter().filter(|s| s.caster == you) {
        let def = &game.obj(s.card).def;
        if !(def.types.contains(&CardType::Instant) || def.types.contains(&CardType::Sorcery)) {
            continue;
        }
        n += 1;
        if s.card == spell {
            break;
        }
    }
    (n - 1).max(0)
}

/// `spell.createCopyOnStack(…, true, stormCount)`: that many copies, new targets asked for each
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    for _ in 0..storm_count(game, you, spell) {
        copy_spell(game, d, &original, you);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
