//! Herald of Anafenza: "Outlast {2}{W}. Whenever you activate Herald of Anafenza's outlast ability, create a 1/1
//! white Warrior creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "HeraldOfAnafenza.HeraldOfAnafenzaTriggeredAbility",
    "new ActivateAbilityTriggeredAbility(new CreateTokenEffect(new WarriorToken()), StaticFilters.FILTER_SPELL_OR_ABILITY, SetTargetPointer.NONE)",
    outlast_activated,
)];

/// `event.getSourceId()` is this permanent and the ability on the stack is its `OutlastAbility` (its own activated
/// ability as a sorcery with the outlast text, below this trigger)
fn outlast_activated(game: &Game, item: &StackItem) -> bool {
    game.stack.iter().any(|s| {
        s.card == item.card
            && s.delayed.is_none()
            && s.ability.is_some_and(|(i, _)| {
                matches!(
                    game.item_def(s).abilities.get(i),
                    Some(AbilityDef::ActivateAsSorceryActivatedAbility { rule, .. }) if rule.starts_with("Outlast ")
                )
            })
    })
}
