//! Raiders' Karve: "Whenever Raiders' Karve attacks, look at the top card of your library. If it's a land card, you
//! may put it onto the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RaidersKarve.RaidersKarveEffect", apply)];

/// `lookAtCards` (titled by the source), then the land's `chooseUse`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: vec![card] });
    if !game.has_type(card, CardType::Land) {
        return Some(());
    }
    let message = format!("Put {} onto the battlefield tapped?", log::object(game, card));
    if choose_use(game, d, you, &message)? && !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped)
    {
        return None;
    }
    Some(())
}
