//! Nicanzil, Current Conductor: "Whenever a creature you control explores a land card, you may put a land card from
//! your hand onto the battlefield tapped. Whenever a creature you control explores a nonland card, put a +1/+1
//! counter on Nicanzil, Current Conductor."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "NicanzilCurrentConductor.NicanzilCurrentConductorTriggeredAbility",
    "new CreatureExploresTriggeredAbility($0) /* isLand: $1 */",
    explored_kind,
)];

/// `EXPLORED` by a creature you control still on the battlefield, with a card revealed: a land for the ability that
/// puts a land (`isLand`), a nonland card for the other
fn explored_kind(game: &Game, t: &StackItem) -> bool {
    let Some((explorer, Some(card))) = t.explored else { return false };
    let o = game.obj(explorer);
    if o.zone != Zone::Battlefield || o.controller != t.controller || !game.has_type(explorer, CardType::Creature) {
        return false;
    }
    let puts_land = t.ability.and_then(|(i, _)| game.item_def(t).abilities.get(i)).is_some_and(|a| {
        matches!(a, AbilityDef::TriggeredAbility { effects, .. }
            if effects.iter().any(|e| matches!(e, EffectDef::PutCardFromHandOntoBattlefieldEffect { .. })))
    });
    game.obj(card).is(CardType::Land) == puts_land
}
