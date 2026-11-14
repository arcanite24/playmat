//! Temple Altisaur: "If a source would deal damage to another Dinosaur you control, prevent all but 1 of that damage."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "TempleAltisaur.TempleAltisaurPreventEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// `DAMAGE_PERMANENT` to another Dinosaur you control (`PreventionEffectImpl.applies`: damage that can be prevented)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(t) = ev.target else { return false };
    t != cx.source
        && game.obj(t).zone == Zone::Battlefield
        && game.has_subtype(t, "Dinosaur")
        && game.obj(t).controller == cx.controller
        && cx.preventable(ev, false)
}

/// `amountToPrevent = damage - 1`, when more than 1 (not consumable)
fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if ev.amount > 1 {
        let n = ev.amount - 1;
        cx.prevent(ev, n);
    }
    false
}
