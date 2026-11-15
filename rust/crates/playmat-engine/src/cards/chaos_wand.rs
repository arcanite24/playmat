//! Chaos Wand: "{4}, {T}: Target opponent exiles cards from the top of their library until they exile an instant or
//! sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast
//! this way on the bottom of that library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChaosWand.ChaosWandEffect", apply)];

/// The opponent's top cards exiled one at a time down to the first instant or sorcery, which you may cast for free
/// (`castSpellWithAttributesForFree`); the ones still exiled are revealed and go to the bottom of that library in a
/// random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let mut exiled = Vec::new();
    for c in game.player(opponent).library.clone() {
        exiled.push(c);
        game.move_to(c, Zone::Exile);
        if game.has_type(c, CardType::Instant) || game.has_type(c, CardType::Sorcery) {
            super::breaching_dragonstorm::cast_for_free(game, d, you, c, |_, _| true)?;
            break;
        }
    }
    let rest: Vec<ObjId> = exiled.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    if !rest.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = rest.iter().map(|c| (*c, Zone::Exile)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    if !put_on_library(game, d, opponent, &rest, false, false) {
        return None;
    }
    Some(())
}
