//! Hedron Alignment: "At the beginning of your upkeep, you may reveal your hand. If you do, you win the game if you
//! own a card named Hedron Alignment in exile, in your hand, in your graveyard, and on the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HedronAlignment.HedronAlignmentEffect", apply)];

/// Java reveals an empty set of cards; then a Hedron Alignment you own on the battlefield, in your hand, in your
/// graveyard and in exile wins the game
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let named = |g: &Game, o: &ObjId| g.obj(*o).owner == you && g.obj(*o).name() == "Hedron Alignment";
    if !game.battlefield.iter().any(|o| named(game, o))
        || !game.player(you).hand.iter().any(|o| named(game, o))
        || !game.player(you).graveyard.iter().any(|o| named(game, o))
        || !game.exile.iter().any(|o| named(game, o))
    {
        return Some(());
    }
    win_game(game, you);
    Some(())
}
