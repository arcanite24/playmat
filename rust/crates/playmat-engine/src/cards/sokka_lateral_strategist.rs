//! Sokka, Lateral Strategist: "Whenever Sokka and at least one other creature attack, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SokkaLateralStrategist.SokkaLateralStrategistTriggeredAbility",
    "new AttacksTriggeredAbility(new DrawCardSourceControllerEffect(1))",
    check,
)];

/// `DECLARED_ATTACKERS` with Sokka among them (the base trigger): two or more attackers in all
fn check(game: &Game, _t: &StackItem) -> bool {
    attackers(game).len() >= 2
}
