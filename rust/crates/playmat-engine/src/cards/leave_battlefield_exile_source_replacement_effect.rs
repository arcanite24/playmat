//! `LeaveBattlefieldExileSourceReplacementEffect`: "If this creature would leave the battlefield, exile it instead
//! of putting it anywhere else." (Geth, Thane of Contracts; Llanowar Greenwidow)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "LeaveBattlefieldExileSourceReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)
.args(&["text"])];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source) && ev.from == Some(Zone::Battlefield) && ev.to != Some(Zone::Exile)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
