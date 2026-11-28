//! `RevealCardsFromLibraryUntilEffect(filter, putPicked, putRemaining)` (Clifftop Lookout): "reveal cards from the top
//! of your library until you reveal a land card. Put that card onto the battlefield tapped and the rest on the bottom
//! of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RevealCardsFromLibraryUntilEffect", apply)
    .args(&["filter", "put_cards", "put_cards"])];

/// The cards from the top up to the first of the filter (all of them if none is) are revealed; that one goes where
/// `putPicked` says, then the revealed cards still in the library where `putRemaining` says
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(filter), [_, picked, rest]) = (cx.filters.first(), cx.args) else { return Some(()) };
    let put = |s: &str| serde_json::from_value::<crate::card::PutCards>(serde_json::Value::String(s.into())).ok();
    let (picked, rest) = (put(picked)?, put(rest)?);
    let you = cx.controller();
    if game.player(you).library.is_empty() {
        return Some(());
    }
    let mut revealed = Vec::new();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        revealed.push(c);
        if game.filter_matches(filter, you, Some(cx.source_id()), c) {
            hit = Some(c);
            break;
        }
    }
    let (src, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: shown });
    if let Some(c) = hit {
        if !put_cards(game, d, you, &[c], picked) {
            return None;
        }
    }
    let left: Vec<ObjId> = revealed.into_iter().filter(|c| Some(*c) != hit && game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &left, rest) {
        return None;
    }
    Some(())
}
