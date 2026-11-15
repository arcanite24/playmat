//! Healing Grace: "Prevent the next 3 damage that would be dealt to any target this turn by a source of your choice.
//! You gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "HealingGrace.HealingGraceEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.on_start(start)];

/// `init`: the spell's target kept first, then "a source of your choice" (`TargetSource.choose`)
fn start(game: &mut Game, d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let mut out: Vec<(Target, u32)> = cx.targets.iter().take(1).copied().collect();
    out.extend(choose_source_resolving(game, d, cx)?);
    Some(out)
}

/// `TargetSource.choose` as `choose_source`, with this spell (still on Java's stack as it resolves, `game.resolving`)
/// among the stack's sources in its place
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

/// Preventable damage to the target from the chosen source, while that source is still the object chosen, with some
/// of the 3 left to prevent
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let (Some((target, _)), Some((Target::Obj(src), zcc))) = (cx.targets.first(), cx.targets.get(1)) else {
        return false;
    };
    let to = match ev.kind {
        EventType::DamagePlayer => Target::Player(ev.player),
        _ => match ev.target {
            Some(o) => Target::Obj(o),
            None => return false,
        },
    };
    cx.preventable(ev, false)
        && to == *target
        && ev.source == Some(*src)
        && game.obj(*src).zcc == *zcc
        && cx.left(game, 3) > 0
}

/// `PreventionEffectImpl.preventDamageAction`: up to what's left of the 3 is prevented
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let left = cx.left(game, 3);
    let prevented = cx.prevent(ev, left);
    cx.set_left(game, left - prevented);
    false
}
