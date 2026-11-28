//! Djinn of Wishes: "{2}{U}{U}, Remove a wish counter from Djinn of Wishes: Reveal the top card of your library. You
//! may play that card without paying its mana cost. If you don't, exile it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DjinnOfWishes.DjinnOfWishesEffect", apply)];

/// The top card revealed; "Play … without paying its mana cost?", then `playCard(card, game, true, …)`: a land only
/// by the turn's active player with a land play left (the timing ignored), a spell cast for free. Not played: into
/// exile.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    let message = format!("Play {} without paying its mana cost?", game.obj(card).def.name);
    let mut played = false;
    if choose_use(game, d, you, &message)? {
        if game.obj(card).is(CardType::Land) {
            if game.active == Some(you) && game.player(you).lands_played < lands_per_turn(game, you) {
                enter_battlefield(game, d, card, you, false);
                game.player_mut(you).lands_played += 1;
                note(game, EventKind::Land, Some(you), Some(card), None, 0);
                played = true;
            }
        } else {
            played = cast_free(game, d, you, card);
        }
        if d.stopped() {
            return None;
        }
    }
    if !played && game.obj(card).zone == Zone::Library {
        game.move_to(card, Zone::Exile);
    }
    Some(())
}
