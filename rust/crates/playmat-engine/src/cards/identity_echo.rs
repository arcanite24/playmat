//! Identity Echo: "{3}{R}: Exile target creature or planeswalker you control. Reveal cards from the top of your
//! library until you reveal a creature or planeswalker card. Put that card onto the battlefield and the rest on the
//! bottom of your library in a random order. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IdentityEcho.IdentityEchoEffect", apply)];

/// the target exiled (still on the battlefield, or nothing happens); then the cards from the top up to the first
/// creature or planeswalker card (all of them if none is) revealed, that one put onto the battlefield, the rest on
/// the bottom in a random order (Esika's Prismatic Bridge)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(t) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    game.move_to(t, Zone::Exile);
    let mut revealed = Vec::new();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        revealed.push(c);
        if game.obj(c).is(CardType::Creature) || game.obj(c).is(CardType::Planeswalker) {
            hit = Some(c);
            break;
        }
    }
    if !revealed.is_empty() {
        let (src, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
        let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: shown });
    }
    if let Some(c) = hit {
        put_onto_battlefield(game, d, c, you);
    }
    let left: Vec<ObjId> =
        revealed.into_iter().filter(|c| Some(*c) != hit && game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &left, false, false) {
        return None;
    }
    Some(())
}
