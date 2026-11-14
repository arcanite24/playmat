//! Wizard's Staff: "If a triggered ability of equipped creature triggers, that ability triggers an additional time."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("WizardsStaff.WizardsStaffEffect", &[EventType::NumberOfTriggers], applies, replace)];

/// the trigger's source is the creature this Equipment is attached to
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.source.is_some_and(|s| game.attached(cx.source) == Some(s))
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
