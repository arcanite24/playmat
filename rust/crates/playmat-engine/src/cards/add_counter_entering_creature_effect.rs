//! `AddCounterEnteringCreatureEffect(mor, counter, outcome)`: "that creature enters with a finality counter on it",
//! made by a card's watcher as a spell it permitted is cast (Noctis, Prince of Lucis; Osteomancer Adept; Leonardo,
//! Sewer Samurai). It lasts the turn, pointed at the spell's card: its source is the spell (`getSpellAbility()`),
//! and its first argument the counter ("finality 1").

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "AddCounterEnteringCreatureEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// `game.getState().addEffect(new AddCounterEnteringCreatureEffect(new MageObjectReference(spell.getCard(), game),
/// counter, …), spell.getSpellAbility())`: `counter` as "name amount"
pub(crate) fn add_for_spell(game: &mut Game, d: &mut dyn Decider, seat: Seat, spell: ObjId, counter: &str) -> Option<()> {
    let e: EffectDef = serde_json::from_value(serde_json::json!(
        {"class": "CardStatic", "handler": "AddCounterEnteringCreatureEffect", "args": [counter]}
    ))
    .ok()?;
    resolve_static(game, d, &e, spell, seat, &[vec![Target::Obj(spell)]])
}

/// that card entering (`mor.refersTo`: the object it is once it leaves the stack), once
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    cx.left(game, 1) > 0 && matches!(cx.targets.first(), Some((Target::Obj(o), _)) if *o == id)
}

/// the counters, put on it by the spell's controller
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let Some(id) = ev.target else { return false };
    if let Some((name, n)) = cx.args.first().and_then(|a| counter_arg(a)) {
        let name = name.to_string();
        add_counters_by(game, id, &name, n, Some(cx.controller));
    }
    cx.set_left(game, 0);
    false
}
