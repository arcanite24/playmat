//! Doc Samson, Super Psychiatrist: "If you would put one or more counters on a permanent you control, put that many
//! plus one of each of those kinds of counters on that permanent instead. {T}: Add X mana of any one color, where X is
//! Doc Samson's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "DocSamsonSuperPsychiatrist.DocSamsonSuperPsychiatristEffect",
    &[EventType::AddCounters],
    applies,
    plus_one,
)];

/// one or more counters you'd put on a permanent you control (entering or on the battlefield)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(o) = ev.target else { return false };
    ev.amount > 0
        && ev.player == cx.controller
        && game.obj(o).zone == Zone::Battlefield
        && game.obj(o).controller == cx.controller
}

/// `CardUtil.overflowInc(amount, 1)`
fn plus_one(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
