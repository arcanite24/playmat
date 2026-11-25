//! Kaya, Geist Hunter's −6: "Exile all cards from all graveyards, then create a 1/1 white Spirit creature token with
//! flying for each card exiled this way."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("KayaGeistHunter.KayaGeistHunterEffect", apply).tokens(&["SpiritWhiteToken"])];

/// every graveyard's cards in the players' table order, moved to exile together; one token for each that's in exile
/// afterwards (none, nothing created)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut cards = Vec::new();
    for p in 0..game.seats() {
        cards.extend(game.player(p).graveyard.iter().copied());
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let n = cards.iter().filter(|c| game.obj(**c).zone == Zone::Exile).count();
    if n > 0 {
        cx.create_tokens(game, d, &cx.tokens[0], n as i32);
    }
    (!d.stopped()).then_some(())
}
