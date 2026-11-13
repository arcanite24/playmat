//! Transmogrify: "Exile target creature. That creature's controller reveals cards from the top of their library until
//! they reveal a creature card. That player puts that card onto the battlefield, then shuffles the rest into their
//! library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Transmogrify.TransmogrifyEffect", apply)];

/// The exiled creature's controller as it last was: the engine keeps no controller for a card that left, so its
/// owner (the same but for a stolen creature)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.item.targets.first().and_then(|g| g.first()).and_then(|t| match t {
        Target::Obj(o) => Some(*o),
        _ => None,
    }) else {
        return Some(());
    };
    let p = if game.obj(o).zone == Zone::Battlefield { game.obj(o).controller } else { game.obj(o).owner };
    let library = game.player(p).library.clone();
    if library.is_empty() {
        return Some(());
    }
    let mut revealed = Vec::new();
    let mut hit = None;
    for c in library {
        revealed.push(c);
        if game.has_type(c, CardType::Creature) {
            hit = Some(c);
            break;
        }
    }
    if let Some(c) = hit {
        if !put_cards(game, d, p, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    if revealed.len() > usize::from(hit.is_some()) {
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
