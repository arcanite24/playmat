//! Reckoner Shakedown: "Target opponent reveals their hand. You may choose a nonland card from it. If you do, that
//! player discards that card. If you don't, put two +1/+1 counters on a creature or Vehicle you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReckonerShakedown.ReckonerShakedownEffect", apply)];

/// `choose(Discard, hand, TargetCard(0, 1, nonland))`; none chosen: a creature or Vehicle you control
/// (`choose(TargetPermanent)` rounds) gets the counters
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let hand = game.player(p).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let nonland: Vec<ObjId> = hand.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = if hand.is_empty() { Vec::new() } else { choose_cards(game, d, you, &hand, &nonland, 0, 1, &rule)? };
    if let Some(&c) = chosen.first() {
        discard_card(game, p, c);
        game.discarded_cards();
        return Some(());
    }
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you && (game.has_type(*o, CardType::Creature) || game.has_subtype(*o, "Vehicle"))
        })
        .collect();
    if mine.is_empty() {
        return Some(());
    }
    if let Some(&o) = choose_cards_rounds(game, d, you, &mine, 1, 1)?.first() {
        add_counters_by(game, o, "+1/+1", 2, Some(you));
    }
    Some(())
}
