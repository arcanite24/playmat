//! Urza Assembles the Titans: "I — Scry 4, then you may reveal the top card of your library. If a planeswalker card is
//! revealed this way, put it into your hand." and "III — You may activate the loyalty abilities of planeswalkers you
//! control twice this turn rather than only once."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("UrzaAssemblesTheTitans.UrzaAssemblesTheTitansScryEffect", scry_reveal),
    Handler::effect("UrzaAssemblesTheTitans.UrzaAssemblesTheTitansLoyaltyEffect", twice),
];

/// `scry(4)`, then with a top card `chooseUse("Reveal the top card of your library?")`, `revealCards`, and a
/// planeswalker card `moveCards` to your hand
fn scry_reveal(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    if !scry(game, d, you, 4, &rule) {
        return None;
    }
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    if !choose_use(game, d, you, "Reveal the top card of your library?")? {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    if game.obj(card).def.is(CardType::Planeswalker) {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

/// `setLoyaltyActivationsAvailable(2)` on the planeswalkers you control, each time effects apply this turn
fn twice(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let turn = game.turn;
    game.player_mut(cx.controller()).loyalty_twice_turn = Some(turn);
    Some(())
}
