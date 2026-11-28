//! Dictate of the Twin Gods: "If a source would deal damage to a permanent or player, it deals double that damage
//! to that permanent or player instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "DictateOfTheTwinGods.DictateOfTheTwinGodsEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

fn applies(_: &Game, _: &StaticCx, _: &Event) -> bool {
    true
}

/// the damage dealt again doubled (`CardUtil.overflowInc`), with this replacement already applied to it
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(ev.amount);
    false
}
