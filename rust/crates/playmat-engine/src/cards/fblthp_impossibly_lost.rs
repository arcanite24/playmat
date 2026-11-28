//! Fblthp, Impossibly Lost: "When one or more of your opponents are dealt combat damage during your turn, draw two
//! cards. If your library has no cards in it, you win the game. Fblthp's owner shuffles him into their library."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "FblthpImpossiblyLost.FblthpImpossiblyLostTriggeredAbility",
        "new OneOrMoreCombatDamagePlayerTriggeredAbility(Zone.BATTLEFIELD, new DrawCardSourceControllerEffect(2), StaticFilters.FILTER_PERMANENT, SetTargetPointer.PLAYER, false)",
        check,
    ),
    Handler::effect("FblthpImpossiblyLost.FblthpImpossiblyLostEffect", win_if_empty),
];

/// `DAMAGED_BATCH_FOR_ONE_PLAYER` of combat damage (the base trigger: during your turn only the permanents you
/// control deal combat damage to players), to an opponent (the player pointer), during your turn
fn check(game: &Game, t: &StackItem) -> bool {
    game.active == Some(t.controller)
        && matches!(t.targets.first().and_then(|g| g.first()), Some(Target::Player(p)) if *p != t.controller)
}

/// "If your library has no cards in it, you win the game"
fn win_if_empty(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.player(cx.controller()).library.is_empty() {
        win_game(game, cx.controller());
    }
    Some(())
}
