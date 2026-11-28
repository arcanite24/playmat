//! Isshin, Two Heavens as One: "If a creature attacking causes a triggered ability of a permanent you control to
//! trigger, that ability triggers an additional time."

use super::*;
use crate::state::{TriggerCause, Zone};

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "IsshinTwoHeavensAsOne.AdditionalTriggersAttackingReplacementEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)
.args(&["one_of:false"])];

/// `AdditionalTriggersAttackingReplacementEffect(false)`: the trigger's source is a permanent you control, and
/// attackers being declared caused it (`ATTACKER_DECLARED`, `DECLARED_ATTACKERS`), whoever's creatures they are
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    matches!(game.trigger_cause, Some(TriggerCause::Attacked))
        && ev.source.is_some_and(|s| game.obj(s).zone == Zone::Battlefield && game.obj(s).controller == cx.controller)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
