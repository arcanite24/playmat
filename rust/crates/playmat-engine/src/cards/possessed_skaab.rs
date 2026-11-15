//! Possessed Skaab: "If Possessed Skaab would die, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("PossessedSkaab.PossessedSkaabDiesEffect", &[EventType::ZoneChange], applies, replace)];

/// `isDiesEvent` of the Skaab itself
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source) && ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
