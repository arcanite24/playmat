//! Hama Pashar, Ruin Seeker: "Room abilities of dungeons you own trigger an additional time."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "HamaPasharRuinSeeker.HamaPasharRuinSeekerEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)];

/// `NUMBER_OF_TRIGGERS` whose source event is `ROOM_ENTERED` by you: the trigger's source is a dungeon in the
/// command zone that you venture through (a dungeon's only triggers are its rooms')
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.source.is_some_and(|s| {
        let o = game.obj(s);
        o.zone == Zone::Command && o.controller == cx.controller && crate::card::dungeon(&o.def.name).is_some()
    })
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
