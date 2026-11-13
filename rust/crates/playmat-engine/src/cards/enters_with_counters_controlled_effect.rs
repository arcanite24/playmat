//! `EntersWithCountersControlledEffect`: "Each other creature you control enters with an additional +1/+1 counter on
//! it." (Grumgully, the Generous; Renata; Oath of Gideon's loyalty; "this turn, …" as an activated ability's: Zameck
//! Guildmage)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EntersWithCountersControlledEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)
.args(&["duration", "filter", "counter", "text"])];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    // excludeSource
    if cx.arg_bool(3) && id == cx.source {
        return false;
    }
    ev.player == cx.controller && cx.matches(game, 0, id)
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let (Some(id), Some((name, n))) = (ev.target, cx.args.get(2).and_then(|a| counter_arg(a))) {
        add_counters_by(game, id, name, n, Some(cx.controller));
    }
    false
}
