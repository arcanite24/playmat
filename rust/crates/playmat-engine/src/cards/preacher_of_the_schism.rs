//! Preacher of the Schism: "Whenever Preacher of the Schism attacks the player with the most life or tied for most
//! life, create a 1/1 white Vampire creature token with lifelink." and "Whenever Preacher of the Schism attacks while
//! you have the most life or are tied for most life, you draw a card and you lose 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "PreacherOfTheSchism.PreacherOfTheSchismFirstTrigger",
        "new AttacksTriggeredAbility(new CreateTokenEffect(new IxalanVampireToken()), false)",
        attacks_most_life,
    ),
    Handler::trigger(
        "PreacherOfTheSchism.PreacherOfTheSchismSecondTrigger",
        "new AttacksTriggeredAbility(new PreacherOfTheSchismDrawEffect(), false)",
        you_most_life,
    ),
    // the second trigger's two effects (`DrawCardSourceControllerEffect(1, true)`, then
    // `LoseLifeSourceControllerEffect(1)`), as one the base trigger can name
    Handler::effect("PreacherOfTheSchism.PreacherOfTheSchismDrawEffect", draw_lose)
        .effects(&["new DrawCardSourceControllerEffect(1, true)", "new LoseLifeSourceControllerEffect(1)"]),
];

/// `PlayerWithTheMostLifePredicate`: no player has more life
fn most_life(game: &Game, p: Seat) -> bool {
    game.players.iter().all(|q| q.life <= game.player(p).life)
}

/// `checkTrigger` is only asked as it triggers: resolving, it holds (Preacher killed in response still makes its
/// token)
fn resolving(game: &Game, t: &StackItem) -> bool {
    game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability)
}

/// It's attacking a player (not a planeswalker or battle: `getPlayer` of one is null), who has the most life
fn attacks_most_life(game: &Game, t: &StackItem) -> bool {
    if resolving(game, t) {
        return true;
    }
    let o = game.obj(t.card);
    match (o.attacking, o.attacking_pw) {
        (Some(p), None) => most_life(game, p),
        _ => false,
    }
}

/// It's attacking, and you have the most life
fn you_most_life(game: &Game, t: &StackItem) -> bool {
    resolving(game, t) || game.obj(t.card).attacking.is_some() && most_life(game, t.controller)
}

fn draw_lose(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, cx.effects);
    Some(())
}
