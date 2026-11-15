//! Way of the Mind Sculptor: "When Way of the Mind Sculptor enters, empower Jace 5. Whenever you activate a loyalty
//! ability, if you removed two or more loyalty counters to activate it, draw a card."

use super::*;
use crate::card::{AbilityDef, CostDef};

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "WayOfTheMindSculptor.WayOfTheMindSculptorTriggeredAbility",
    "new ActivatePlaneswalkerLoyaltyAbilityTriggeredAbility(new DrawCardSourceControllerEffect(1), SetTargetPointer.SPELL)",
    removed_two,
)];

/// `checkTrigger`: the loyalty ability activated (the topmost of its source's abilities you control on the stack)
/// has `PayLoyaltyCost`s removing two or more counters; resolving, it holds
fn removed_two(game: &Game, t: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability) {
        return true;
    }
    let Some(Target::Obj(src)) = t.targets.first().and_then(|g| g.first()).copied() else { return false };
    let Some(item) = game.stack.iter().rev().find(|i| i.card == src && i.ability.is_some() && i.controller == t.controller)
    else {
        return false;
    };
    let Some(AbilityDef::LoyaltyAbility { cost, .. }) = item.ability.and_then(|(k, _)| game.item_def(item).abilities.get(k))
    else {
        return false;
    };
    let removed: i32 = cost
        .iter()
        .map(|c| match c {
            CostDef::PayLoyaltyCost { amount } if *amount < 0 => -amount,
            _ => 0,
        })
        .sum();
    removed >= 2
}
