//! Tablet of Discovery, Ark of Hunger: "mill a card. You may play that card this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TabletOfDiscovery.TabletOfDiscoveryEffect", apply),
    Handler::effect("ArkOfHunger.ArkOfHungerEffect", apply),
];

/// `millCards(1)`, then `CardUtil.makeCardPlayable(…, Duration.EndOfTurn, …)` for the card milled, in the graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top = game.player(you).library.first().copied();
    cx.run(game, d, &[EffectDef::MillCardsControllerEffect { amount: crate::card::Amount::Fixed(1) }]);
    let Some(card) = top else { return Some(()) };
    if game.obj(card).zone == Zone::Graveyard {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
    }
    Some(())
}
