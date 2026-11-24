//! Liliana, Waker of the Dead: "+1: Each player discards a card. Each opponent who can't loses 3 life. −7: You get
//! an emblem with "At the beginning of combat on your turn, put target creature card from a graveyard onto the
//! battlefield under your control. It gains haste.""

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LilianaWakerOfTheDead.LilianaWakerOfTheDeadDiscardEffect", discard_or_lose),
    Handler::effect("ReturnCreatureFromGraveyardToBattlefieldAndGainHasteEffect", return_with_haste).effects(&[
        "new ReturnFromGraveyardToBattlefieldTargetEffect()",
        "new GainAbilityTargetEffect(HasteAbility.getInstance(), Duration.WhileOnBattlefield)",
    ]),
];

/// `moveCards` the target onto the battlefield under your control; the permanent it became gains haste for as long
/// as it stays (`Duration.Custom`, nothing ends it)
fn return_with_haste(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(card) = cx.obj(0) else { return Some(()) };
    cx.run(game, d, &cx.effects[..1]);
    if game.obj(card).zone == Zone::Battlefield {
        cx.run_on(game, d, &cx.effects[1..], vec![Target::Obj(card)]);
    }
    Some(())
}

/// Each player in turn order from you chooses a card to discard (`TargetDiscard`, none with an empty hand); then,
/// in the same order, each discards it, and an opponent who discarded nothing loses 3 life
fn discard_or_lose(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let seats: Vec<Seat> = (0..game.seats()).map(|k| (you + k) % game.seats()).collect();
    let mut chosen: Vec<Vec<ObjId>> = Vec::new();
    for &p in &seats {
        let hand = game.player(p).hand.clone();
        let n = hand.len().min(1);
        chosen.push(if n == 0 { Vec::new() } else { choose_cards(game, d, p, &hand, &hand, n, n, "discard")? });
    }
    for (p, cards) in seats.into_iter().zip(chosen) {
        let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Hand).collect();
        for c in &cards {
            discard_card(game, p, *c);
        }
        if !cards.is_empty() {
            game.discarded_cards();
        }
        if p != you && cards.is_empty() {
            lose_life(game, p, 3);
        }
    }
    Some(())
}
