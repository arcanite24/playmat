//! Distended Mindbender: "When you cast this spell, target opponent reveals their hand. You choose from it a nonland
//! card with mana value 3 or less and a card with mana value 4 or greater. That player discards those cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DistendedMindbender.DistendedMindbenderEffect", apply)];

/// the hand revealed; one `TargetCard` choice each (`Player.choose` from the hand), then both discarded together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponent = cx.player(0)?;
    let hand = game.player(opponent).hand.clone();
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: hand.iter().map(|c| (*c, Zone::Hand)).collect(),
        });
    }
    let rule = item_rule(game, cx.item);
    let low: Vec<ObjId> = hand
        .iter()
        .copied()
        .filter(|c| {
            let def = &game.obj(*c).def;
            def.mana_value() < 4 && !def.is(CardType::Land)
        })
        .collect();
    let high: Vec<ObjId> = hand.iter().copied().filter(|c| game.obj(*c).def.mana_value() > 3).collect();
    let mut to_discard = Vec::new();
    for possible in [low, high] {
        if possible.is_empty() {
            continue;
        }
        to_discard.extend(choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?);
    }
    if to_discard.is_empty() {
        return Some(());
    }
    for c in &to_discard {
        discard_card(game, opponent, *c);
    }
    game.discarded_cards();
    Some(())
}
