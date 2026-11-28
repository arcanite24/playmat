//! `LeaveBattlefieldExileTargetReplacementEffect`: "If it would leave the battlefield, exile it instead of putting it
//! anywhere else." (Whip of Erebos), resolving after the effect that returned it

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "LeaveBattlefieldExileTargetReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)
.args(&["text"])];

/// The target pointer as the effect started (the permanent the returned card became: `FirstTargetPointer.init` after
/// the return) leaving the battlefield for anywhere but exile. (Java's lasts for one use; this one until the turn's
/// cleanup, which the next end step's exile comes before.)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target.is_some()
        && ev.target == cx.target_obj(game, 0)
        && ev.from == Some(Zone::Battlefield)
        && ev.to != Some(Zone::Exile)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
