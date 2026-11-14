//! Dakra Mystic: "{U}, {T}: Each player reveals the top card of their library. You may put the revealed cards into
//! their owners' graveyards. If you don't, each player draws a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DakraMystic.DakraMysticEffect", apply)];

/// Each player in turn order from you reveals their top card; you choose (`chooseUse`) to mill them all, else each
/// player draws a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|k| (you + k) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let src = cx.source_id();
    for &p in &players {
        if let Some(&top) = game.player(p).library.first() {
            game.revealed.push(crate::state::Revealed {
                source: src,
                zcc: game.obj(src).zcc,
                paren: false,
                cards: vec![(top, Zone::Library)],
            });
        }
    }
    if choose_use(game, d, you, "Put revealed cards into graveyard?")? {
        for &p in &players {
            if let Some(&top) = game.player(p).library.first() {
                game.move_to(top, Zone::Graveyard);
            }
        }
    } else {
        for &p in &players {
            draw(game, p, 1);
        }
    }
    Some(())
}
