//! Wildwood Escort: "When Wildwood Escort enters, return target creature or battle card from your graveyard to your
//! hand. If Wildwood Escort would die, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("WildwoodEscort.WildwoodEscortEffect", &[EventType::ZoneChange], applies, replace)];

/// `isDiesEvent` of the source itself: from the battlefield to a graveyard
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source) && ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
