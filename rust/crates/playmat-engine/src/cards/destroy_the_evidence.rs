//! Destroy the Evidence: "Destroy target land. Its controller reveals cards from the top of their library until they
//! reveal a land card, then puts those cards into their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DestroyTheEvidence.DestroyTheEvidenceEffect", apply)];

/// The land's controller (as it last was) reveals down to a land, and all of those go to the graveyard together
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.item.targets.first()?.first().and_then(|t| if let Target::Obj(o) = t { Some(*o) } else { None })?;
    let p = game.obj(o).controller;
    let mut cards = Vec::new();
    for c in game.player(p).library.clone() {
        cards.push(c);
        if game.obj(c).is(CardType::Land) {
            break;
        }
    }
    if !cards.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    for c in cards {
        game.move_to(c, Zone::Graveyard);
    }
    Some(())
}
