//! Cloud, Midgar Mercenary: "As long as Cloud is equipped, if an ability of Cloud or an Equipment attached to it
//! triggers, that ability triggers an additional time."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "CloudMidgarMercenary.CloudMidgarMercenaryEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)];

/// Cloud is equipped, and the trigger's source is Cloud or an Equipment attached to it
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let equipped = |g: &Game| {
        g.battlefield.iter().any(|a| g.attached(*a) == Some(cx.source) && g.has_subtype(*a, "Equipment"))
    };
    let Some(src) = ev.source else { return false };
    equipped(game)
        && (src == cx.source || (game.attached(src) == Some(cx.source) && game.has_subtype(src, "Equipment")))
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
