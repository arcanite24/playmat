//! Ghostly Pilferer: "Whenever Ghostly Pilferer becomes untapped, you may pay {2}. If you do, draw a card. Whenever
//! an opponent casts a spell from anywhere other than their hand, draw a card. Discard a card: Ghostly Pilferer
//! can't be blocked this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GhostlyPilferer.GhostlyPilfererTriggeredAbility",
    "new SpellCastAllTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_SPELL_A, false, SetTargetPointer.PLAYER)",
    opponent_not_from_hand,
)];

/// `checkTrigger`: the spell cast (the latest spell on the stack, its caster the trigger's player pointer) is an
/// opponent's, cast from anywhere other than their hand (`event.getZone() != Zone.HAND`)
fn opponent_not_from_hand(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Player(p)) = t.targets.first().and_then(|ts| ts.first()).copied() else {
        return false;
    };
    if p == t.controller {
        return false;
    }
    let spell = game.stack.iter().rev().find(|s| s.ability.is_none() && game.obj(s.card).zone == Zone::Stack);
    spell.is_some_and(|s| s.controller == p && game.obj(s.card).cast_from != Some(Zone::Hand))
}
