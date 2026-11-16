//! Gyruda, Doom of Depths: "When Gyruda, Doom of Depths enters the battlefield, each player mills four cards. Put a
//! creature card with an even mana value from among the milled cards onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GyrudaDoomOfDepths.GyrudaDoomOfDepthsEffect", apply)];

/// Each player in range from you mills four; then up to one creature card with an even mana value among the milled
/// cards, wherever they went (`TargetCard(0, 1, Zone.ALL, …)`, not targeted), onto the battlefield under your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cards.extend(game.player(p).library.iter().take(4).copied());
    }
    cx.run(game, d, &[EffectDef::MillCardsEachPlayerEffect { amount: Amount::Fixed(4), who: "any".into() }]);
    if d.stopped() {
        return None;
    }
    if cards.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.is(CardType::Creature) && o.def.mana_value().is_multiple_of(2) && o.zone != Zone::Library
        })
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &possible, 0, 1, &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
