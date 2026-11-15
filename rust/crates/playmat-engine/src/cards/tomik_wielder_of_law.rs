//! Tomik, Wielder of Law: "Affinity for planeswalkers. Flying, vigilance. Whenever an opponent attacks with creatures,
//! if two or more of those creatures are attacking you and/or planeswalkers you control, that opponent loses 3 life
//! and you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "TomikWielderOfLaw.TomikWielderOfLawTriggeredAbility",
        "new AureliaTheLawAboveTriggeredAbility(new TomikWielderOfLawEffect(), 1)",
        opponent_attacks_you_with_two,
    ),
    Handler::effect("TomikWielderOfLaw.TomikWielderOfLawEffect", apply),
];

/// `DECLARED_ATTACKERS` with an opponent of yours the active player, and (the intervening "if") two or more of the
/// attackers have you as their defending player (attacking you or a planeswalker you control)
fn opponent_attacks_you_with_two(game: &Game, t: &StackItem) -> bool {
    let you = t.controller;
    game.active.is_some_and(|a| a != you) && attackers(game).iter().filter(|a| game.obj(**a).attacking == Some(you)).count() >= 2
}

/// `LoseLifeTargetEffect(3)` at the attacking player (`FixedTarget`), then `DrawCardSourceControllerEffect(1)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if let Some(attacker) = game.active.filter(|a| *a != you) {
        lose_life(game, attacker, 3);
    }
    cx.run(game, d, &[EffectDef::DrawCardSourceControllerEffect { amount: crate::card::Amount::Fixed(1) }]);
    Some(())
}
