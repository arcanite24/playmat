//! Deflecting Palm: "The next time a source of your choice would deal damage to you this turn, prevent that damage.
//! If damage is prevented this way, Deflecting Palm deals that much damage to that source's controller."
//! (`PreventNextDamageFromChosenSourceEffect` with `DeflectingPalmPreventionApplier`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "DeflectingPalm.PreventNextDamageFromChosenSourceEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["text", "text", "one_of:DeflectingPalmPreventionApplier.instance"])
.on_start(choose_source)];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    // toYou
    cx.preventable(ev, false)
        && ev.kind == EventType::DamagePlayer
        && ev.player == cx.controller
        && cx.left(game, 1) > 0
        && cx.targets.first().is_some_and(|(t, _)| Some(*t) == ev.source.map(Target::Obj))
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let prevented = cx.prevent(ev, i32::MAX);
    // `discard()`
    cx.set_left(game, 0);
    if prevented > 0 {
        let Some((Target::Obj(chosen), _)) = cx.targets.first() else { return false };
        let controller = source_controller(game, *chosen);
        deal_damage(game, cx.source, Target::Player(controller), prevented, false);
    }
    false
}

/// `game.getControllerId(id)`: a spell's controller on the stack, else the object's
fn source_controller(game: &Game, id: ObjId) -> Seat {
    game.stack.iter().find(|i| i.card == id && i.ability.is_none()).map_or(game.obj(id).controller, |i| i.controller)
}
