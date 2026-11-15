//! Imminent Doom: "Imminent Doom enters with a doom counter on it. Whenever you cast a spell with mana value equal to
//! the number of doom counters on Imminent Doom, Imminent Doom deals that much damage to any target. Then put a doom
//! counter on Imminent Doom."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ImminentDoom.ImminentDoomTriggeredAbility",
        "new SpellCastControllerTriggeredAbility(new ImminentDoomEffect(), false)",
        doom_count,
    ),
    Handler::effect("ImminentDoom.ImminentDoomEffect", apply),
];

/// the spell you cast has mana value equal to the doom counters on this, which is on the battlefield
fn doom_count(game: &Game, item: &StackItem) -> bool {
    let Some(spell) = item.cast_spell else { return false };
    game.obj(item.card).zone == Zone::Battlefield
        && game.obj(spell).def.mana_value() as i32 == game.obj(item.card).counter("doom")
}

/// This still on the battlefield: that much damage (the spell's mana value, the count it triggered on) to the target,
/// then a doom counter on it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).zone != Zone::Battlefield {
        return Some(());
    }
    let n = cx.item.cast_spell.map(|s| game.obj(s).def.mana_value() as i32).unwrap_or(0);
    if let Some(t) = cx.target(0) {
        deal_damage(game, src, t, n, false);
    }
    add_counters_by(game, src, "doom", 1, Some(cx.controller()));
    Some(())
}
