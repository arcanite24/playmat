//! Rat King, Verminister: "{T}, Sacrifice three Rats: Return target creature card and all other cards with the same
//! name as that card from your graveyard to the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RatKingVerminister.RatKingVerministerEffect", apply)];

/// The target and every card in your graveyard sharing its name, in one move
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    let name = game.obj(card).def.name.clone();
    let mut cards: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|&c| c != card && game.obj(c).def.name == name).collect();
    cards.insert(0, card);
    if !put_cards(game, d, you, &cards, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}
