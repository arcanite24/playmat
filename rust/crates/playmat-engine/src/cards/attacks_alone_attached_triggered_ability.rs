//! `AttacksAloneAttachedTriggeredAbility`: "Whenever equipped creature attacks alone, …" (S.H.I.E.L.D. Spy Kit,
//! Sigil of Valor): `AttacksAttachedTriggeredAbility` while `Combat.attacksAlone` (one attacker). The transpiler
//! writes the attached trigger with this check.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "AttacksAloneAttachedTriggeredAbility",
    "new AttacksAttachedTriggeredAbility($0)",
    alone,
)];

fn alone(game: &Game, _: &StackItem) -> bool {
    attackers(game).len() == 1
}
