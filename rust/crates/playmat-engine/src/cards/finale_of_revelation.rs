//! Finale of Revelation: "Draw X cards. If X is 10 or more, instead shuffle your graveyard into your library, draw X
//! cards, untap up to five lands, and you have no maximum hand size for the rest of the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FinaleOfRevelation.FinaleOfRevelationEffect", apply)
    .effects(&["new UntapLandsEffect(5)"])];

/// X below 10: draw X. Otherwise the graveyard goes on top of the library (`putCardsOnTopOfLibrary`, in graveyard
/// order), the library is shuffled, X drawn, `UntapLandsEffect(5)`, and no maximum hand size
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x.max(0) as usize;
    if x < 10 {
        draw(game, you, x);
        return Some(());
    }
    for c in game.player(you).graveyard.clone() {
        to_library(game, c, true);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    draw(game, you, x);
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    game.player_mut(you).no_max_hand_size = true;
    Some(())
}
