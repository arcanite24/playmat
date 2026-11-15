//! Gloomshrieker: "Menace. When this creature enters, return target permanent card from your graveyard to your hand.
//! If this creature would die, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("Gloomshrieker.GloomshriekerEffect", &[EventType::ZoneChange], applies, replace)];

/// `isDiesEvent` of the source itself
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source) && ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
