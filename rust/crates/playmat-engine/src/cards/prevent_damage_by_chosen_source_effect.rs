//! `PreventDamageByChosenSourceEffect()`: "Prevent all damage a source of your choice would deal this turn" (Pay No
//! Heed; the `(filter, true)` combat form is the engine's `PreventCombatDamageByChosenEffect`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventDamageByChosenSourceEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.on_start(choose_source_resolving)];

/// `init`: `TargetSource.choose`, with the resolving spell (still on Java's stack, `game.resolving`) among the
/// stack's sources in its place (Healing Grace's)
fn choose_source_resolving(game: &mut Game, d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let mut can: Vec<ObjId> = game.stack.iter().filter(|i| i.ability.is_none()).map(|i| i.card).collect();
    if let Some((at, card)) = game.resolving {
        if !can.contains(&card) {
            can.insert(at.min(can.len()), card);
        }
    }
    can.extend(game.battlefield.iter().copied());
    for p in 0..game.seats() {
        can.extend(game.player(p).graveyard.iter().copied());
    }
    can.extend(game.exile.iter().copied());
    can.dedup();
    if can.is_empty() {
        return Some(Vec::new());
    }
    let chosen = choose_cards(game, d, cx.controller, &can, &can, 1, 1, "source of your choice")?;
    Some(chosen.into_iter().map(|o| (Target::Obj(o), game.obj(o).zcc)).collect())
}

/// preventable damage from the chosen source, while it's still the object chosen (`MageObjectReference.refersTo`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some((Target::Obj(src), zcc)) = cx.targets.first() else { return false };
    cx.preventable(ev, false) && ev.source == Some(*src) && game.obj(*src).zcc == *zcc
}

/// all of it (`Integer.MAX_VALUE` to prevent)
fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
