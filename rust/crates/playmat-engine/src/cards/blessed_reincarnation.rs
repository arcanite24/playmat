//! Blessed Reincarnation: "Exile target creature an opponent controls. That player reveals cards from the top of
//! their library until a creature card is revealed. The player puts that card onto the battlefield, then shuffles
//! the rest into their library. Rebound"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlessedReincarnation.BlessedReincarnationEffect", apply)];

/// The target still on the battlefield is exiled; its controller then goes through their library from the top until
/// a creature card, which they put onto the battlefield; the cards gone through are revealed, and with more than one
/// of them the library is shuffled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(it) = cx.obj(0) else { return Some(()) };
    if game.obj(it).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(it).controller;
    game.move_to(it, Zone::Exile);
    let library = game.player(p).library.clone();
    if library.is_empty() {
        return Some(());
    }
    let mut revealed = Vec::new();
    for c in library {
        revealed.push(c);
        if game.obj(c).is(CardType::Creature) {
            if !put_cards(game, d, p, &[c], crate::card::PutCards::Battlefield) {
                return None;
            }
            break;
        }
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: revealed.iter().map(|c| (*c, Zone::Library)).collect(),
    });
    if revealed.len() > 1 {
        cx.run_as(game, d, &[EffectDef::ShuffleLibrarySourceEffect], p);
    }
    Some(())
}
